use anyhow::{Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Profile {
    name: String,
    prefilter: String,
    pattern: String,
    version: String,
    field_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    String,
    Int,
    Float,
    DateTime,
    Enum,
    Json,
}

#[derive(Debug, Clone)]
struct MacroInvocation {
    field: Option<String>,
    name: String,
    args: Vec<String>,
}

fn split_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quote: Option<char> = None;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = in_quote {
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    cur.push(next);
                    chars.next();
                }
            } else if c == q {
                in_quote = None;
            } else {
                cur.push(c);
            }
        } else {
            if c == '\'' || c == '"' {
                in_quote = Some(c);
            } else if c == ',' {
                args.push(cur.trim().to_string());
                cur.clear();
            } else {
                cur.push(c);
            }
        }
    }
    if !cur.trim().is_empty() {
        args.push(cur.trim().to_string());
    }
    args.into_iter()
        .map(|a| {
            if (a.starts_with('"') && a.ends_with('"'))
                || (a.starts_with('\'') && a.ends_with('\''))
            {
                a[1..a.len() - 1].to_string()
            } else {
                a
            }
        })
        .collect()
}

fn parse_macro_invocation(s: &str) -> Result<MacroInvocation> {
    let s = s.trim();
    if s.is_empty() {
        bail!("empty macro token");
    }
    if let Some(paren_start) = s.find('(') {
        let before = &s[..paren_start];
        let after = &s[paren_start + 1..];
        if !after.ends_with(')') {
            bail!("unclosed parenthesis in macro invocation: {}", s);
        }
        let inside = &after[..after.len() - 1];
        if let Some(colon_pos) = before.find(':') {
            let field = before[..colon_pos].trim().to_string();
            let name = before[colon_pos + 1..].trim().to_string();
            let args = split_args(inside);
            Ok(MacroInvocation {
                field: Some(field),
                name,
                args,
            })
        } else {
            let name = before.trim().to_string();
            let args = split_args(inside);
            Ok(MacroInvocation {
                field: None,
                name,
                args,
            })
        }
    } else if let Some(colon_pos) = s.find(':') {
        let left = s[..colon_pos].trim();
        let right = s[colon_pos + 1..].trim();
        if right
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            || right.contains('-')
            || right.contains(',')
        {
            let name = left.to_string();
            let args = vec![right.to_string()];
            Ok(MacroInvocation {
                field: None,
                name,
                args,
            })
        } else {
            let field = left.to_string();
            let name = right.to_string();
            Ok(MacroInvocation {
                field: Some(field),
                name,
                args: vec![],
            })
        }
    } else {
        Ok(MacroInvocation {
            field: None,
            name: s.to_string(),
            args: vec![],
        })
    }
}

fn expand_builtin_macro(name: &str, args: &[String]) -> Result<(String, Option<FieldType>)> {
    match name.to_lowercase().as_str() {
        "number" | "num" => {
            if args.is_empty() {
                Ok((r"\d+".to_string(), Some(FieldType::Int)))
            } else {
                let a = &args[0];
                if let Some(pos) = a.find('-') {
                    let min = a[..pos].trim();
                    let max = a[pos + 1..].trim();
                    Ok((format!(r"\d{{{},{}}}", min, max), Some(FieldType::Int)))
                } else if a.chars().all(|c| c.is_ascii_digit()) {
                    Ok((format!(r"\d{{{}}}", a), Some(FieldType::Int)))
                } else {
                    bail!("invalid number macro arg: {}", a)
                }
            }
        }
        "string" | "str" => Ok((r".+?".to_string(), Some(FieldType::String))),
        "var_name" | "ident" => Ok((
            r"[A-Za-z_][A-Za-z0-9_]*".to_string(),
            Some(FieldType::String),
        )),
        "uuid" => Ok((
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"
                .to_string(),
            Some(FieldType::String),
        )),
        "enum" => {
            if args.is_empty() {
                bail!("enum macro requires comma-separated values");
            }
            let vals = args.join(",");
            let items: Vec<String> = vals.split(',').map(|v| regex::escape(v.trim())).collect();
            Ok((format!(r"(?:{})", items.join("|")), Some(FieldType::Enum)))
        }
        "datetime" | "ts" => {
            if !args.is_empty() {
                Ok((r"\S+".to_string(), Some(FieldType::DateTime)))
            } else {
                Ok((r"\S+".to_string(), Some(FieldType::DateTime)))
            }
        }
        "any" => Ok((r".+?".to_string(), Some(FieldType::String))),
        _ => bail!("unknown macro '{}'", name),
    }
}

pub fn expand_macros(pattern: &str) -> Result<(String, Vec<String>, HashMap<String, FieldType>)> {
    let mut out = String::with_capacity(pattern.len());
    let mut i = 0usize;
    let bytes = pattern.as_bytes();
    let mut auto_idx = 0usize;
    let mut field_names: Vec<String> = Vec::new();
    let mut type_hints: HashMap<String, FieldType> = HashMap::new();
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if i > 0 && bytes[i - 1] == b'\\' {
                out.pop();
                out.push_str("{{");
                i += 2;
                continue;
            }
            let mut j = i + 2;
            let mut found = false;
            while j + 1 < bytes.len() {
                if bytes[j] == b'}' && bytes[j + 1] == b'}' {
                    found = true;
                    break;
                }
                j += 1;
            }
            if !found {
                bail!("unclosed '{{{{' in pattern");
            }
            let content = &pattern[i + 2..j];
            let inv = parse_macro_invocation(content)?;
            let (frag, hint) = expand_builtin_macro(&inv.name, &inv.args)?;
            let field_name = if let Some(f) = inv.field {
                f
            } else {
                auto_idx += 1;
                format!("auto_{}_{}", auto_idx, inv.name)
            };
            let capture = format!("(?P<{}>{})", field_name, frag);
            out.push_str(&capture);
            field_names.push(field_name.clone());
            if let Some(h) = hint {
                type_hints.insert(field_name, h);
            }
            i = j + 2;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    Ok((out, field_names, type_hints))
}

#[derive(Debug, Clone)]
pub struct Scanner {
    regex: Regex,
    pub field_names: Vec<String>,
    pub type_hints: HashMap<String, FieldType>,
}

impl Scanner {
    pub fn new(pattern: String) -> Self {
        let (expanded, field_names, type_hints) = expand_macros(&pattern).unwrap();
        let regex = Regex::new(&expanded).unwrap();
        Self {
            regex,
            field_names,
            type_hints,
        }
    }

    pub fn scan(&self, line: &str) -> Option<Vec<String>> {
        if let Some(captures) = self.regex.captures(line) {
            let mut values = Vec::new();
            for name in &self.field_names {
                if let Some(value) = captures.name(name) {
                    values.push(value.as_str().to_string());
                } else {
                    values.push(String::new());
                }
            }
            Some(values)
        } else {
            None
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    #[test]
    fn test_expand_shorthand_number() {
        let pat = "qty: {{number:3-5}}";
        let (expanded, fields, _) = expand_macros(pat).unwrap();
        assert!(expanded.contains(r"\d{3,5}"));
        assert_eq!(fields.len(), 1);
    }

    #[test]
    fn test_auto_named_capture() {
        let pat = "count={{number}} items";
        let (expanded, fields, _) = expand_macros(pat).unwrap();
        assert!(expanded.contains("(?P<auto_1_number>"));
        assert_eq!(fields, vec!["auto_1_number".to_string()]);
    }

    #[test]
    fn test_named_field_capture() {
        let pat = "user {{name:var_name}} logged";
        let (expanded, fields, _) = expand_macros(pat).unwrap();
        assert!(expanded.contains("(?P<name>"));
        assert_eq!(fields, vec!["name".to_string()]);
    }

    #[test]
    fn test_datetime_macro_hint() {
        let pat = "{{ts:datetime(\"%Y-%m-%d %H:%M:%S\")}} - msg";
        let (_expanded, fields, hints) = expand_macros(pat).unwrap();
        assert_eq!(fields.len(), 1);
        let hint = hints.get(&fields[0]).unwrap();
        assert_eq!(*hint, FieldType::DateTime);
    }

    #[test]
    fn test_scanner_integration() {
        let pattern = r"^\[(?P<time>\w{3} \w{3} \d{1,2})\] \[(?P<level>[^\]]+)\] (?P<message>.*)$"
            .to_string();
        let scanner = Scanner::new(pattern);
        let line = "[Mon Jan 1] [INFO] hello";
        let res = scanner.scan(line);
        assert!(res.is_some());
    }
}

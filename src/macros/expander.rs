//! Macro expansion logic.
//!
//! Expands macro invocations into regex patterns with field type hints.

use std::collections::HashMap;

use crate::error::{Error, Result};

use crate::macros::parser::{CustomMacro, parse_macro_invocation};
use crate::types::{DateTime, FieldType};

/// Expand a single macro - checks custom macros first, then falls back to builtins.
fn expand_macro(
    name: &str,
    args: &[String],
    custom_macros: Option<&[CustomMacro]>,
) -> Result<(String, Option<FieldType>)> {
    // Check custom macros first
    if let Some(customs) = custom_macros
        && let Some(custom) = customs.iter().find(|m| m.name == name)
    {
        return Ok((custom.pattern.clone(), custom.type_hint.clone()));
    }
    // Fall back to builtin
    expand_builtin_macro(name, args)
}

/// Expand a built-in macro into a regex fragment and optional field type hint.
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
                    Err(Error::Macro(format!("invalid number macro arg: {}", a)))
                }
            }
        }
        "string" | "str" => Ok((r".+?".to_string(), Some(FieldType::String))),
        "float" | "double" => Ok((
            r"[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?".to_string(),
            Some(FieldType::Float),
        )),
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
                return Err(Error::Macro(
                    "enum macro requires comma-separated values".into(),
                ));
            }
            let vals = args.join(",");
            let items: Vec<String> = vals.split(',').map(|v| regex::escape(v.trim())).collect();
            Ok((format!(r"(?:{})", items.join("|")), Some(FieldType::Enum)))
        }
        "datetime" | "ts" => {
            if args.is_empty() {
                Ok((
                    r"\S+".to_string(),
                    Some(FieldType::DateTime(DateTime::new(Some(vec![
                        "%Y-%m-%dT%H:%M:%S%.f".to_string(),
                    ])))),
                ))
            } else {
                // translate strftime-like format string(s) into a regex fragment
                let mut frags = Vec::new();
                for fmt in args.iter() {
                    let frag = format_to_regex(fmt)?;
                    frags.push(frag);
                }
                if frags.len() == 1 {
                    Ok((
                        frags.into_iter().next().unwrap(),
                        Some(FieldType::DateTime(DateTime::new(Some(args.into())))),
                    ))
                } else {
                    Ok((
                        format!("(?:{})", frags.join("|")),
                        Some(FieldType::DateTime(DateTime::new(Some(args.into())))),
                    ))
                }
            }
        }
        "any" => Ok((r".+?".to_string(), Some(FieldType::String))),
        _ => Err(Error::Macro(format!("unknown macro '{}'", name))),
    }
}

/// Convert a strftime format string to a regex pattern.
fn format_to_regex(fmt: &str) -> Result<String> {
    // naive strftime -> regex translator for common directives
    // supports: %Y, %y, %m, %d, %H, %M, %S, %f, %z, %Z, %b, %B, %a, %A
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(d) = chars.next() {
                match d {
                    'Y' => out.push_str(r"\d{4}"),
                    'y' => out.push_str(r"\d{2}"),
                    'm' => out.push_str(r"\d{2}"),
                    'd' => out.push_str(r"\d{2}"),
                    'H' => out.push_str(r"\d{2}"),
                    'M' => out.push_str(r"\d{2}"),
                    'S' => out.push_str(r"\d{2}"),
                    'f' => out.push_str(r"\d+"),
                    'z' => out.push_str(r"[+-]\d{4}"),
                    'Z' => out.push_str(r"[A-Za-z/_+-]+"),
                    'b' | 'B' => out.push_str(r"[A-Za-z]+"),
                    'a' | 'A' => out.push_str(r"[A-Za-z]+"),
                    '%' => out.push('%'),
                    other => {
                        return Err(Error::Macro(format!(
                            "unsupported datetime directive: %{}",
                            other
                        )));
                    }
                }
            } else {
                return Err(Error::Macro(
                    "incomplete datetime format string: ends with %".into(),
                ));
            }
        } else {
            // escape regex metacharacters in literals
            let esc = regex::escape(&c.to_string());
            out.push_str(&esc);
        }
    }
    Ok(out)
}

/// Expand all macros in a pattern string.
///
/// Arguments:
/// - `pattern`: The pattern string containing macros like `{{field:macro}}`
/// - `custom_macros`: Optional slice of custom macros to check before builtins
///
/// Returns:
/// - The expanded regex pattern
/// - A list of field names in order
/// - A map of field names to their type hints
pub fn expand_macros(
    pattern: &str,
    custom_macros: Option<&[CustomMacro]>,
) -> Result<(String, Vec<String>, HashMap<String, FieldType>)> {
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
                return Err(Error::Macro("unclosed '{{' in pattern".into()));
            }
            let content = &pattern[i + 2..j];
            let inv = parse_macro_invocation(content)?;
            let (frag, hint) = expand_macro(&inv.name, &inv.args, custom_macros)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_shorthand_number() {
        let pat = "qty: {{number:3-5}}";
        let (expanded, fields, _) = expand_macros(pat, None).unwrap();
        assert!(expanded.contains(r"\d{3,5}"));
        assert_eq!(fields.len(), 1);
    }

    #[test]
    fn test_auto_named_capture() {
        let pat = "count={{number}} items";
        let (expanded, fields, _) = expand_macros(pat, None).unwrap();
        assert!(expanded.contains("(?P<auto_1_number>"));
        assert_eq!(fields, vec!["auto_1_number".to_string()]);
    }

    #[test]
    fn test_named_field_capture() {
        let pat = "user {{name:var_name}} logged";
        let (expanded, fields, _) = expand_macros(pat, None).unwrap();
        assert!(expanded.contains("(?P<name>"));
        assert_eq!(fields, vec!["name".to_string()]);
    }

    #[test]
    fn test_datetime_macro_hint() {
        let pat = "{{ts:datetime(\"%Y-%m-%d %H:%M:%S\")}} - msg";
        let (_expanded, fields, hints) = expand_macros(pat, None).unwrap();
        assert_eq!(fields.len(), 1);
        let hint = hints.get(&fields[0]).unwrap();
        assert_eq!(
            *hint,
            FieldType::DateTime(DateTime::new(Some(vec!["%Y-%m-%d %H:%M:%S".to_string()])))
        );
    }

    #[test]
    fn test_datetime_multiple_formats() {
        let pat = "{{ts:datetime(\"%Y-%m-%d %H:%M:%S\",\"%d/%b/%Y:%H:%M:%S\")}} - msg";
        let (expanded, fields, hints) = expand_macros(pat, None).unwrap();
        assert!(expanded.contains("|"));
        assert_eq!(fields.len(), 1);
        let hint = hints.get(&fields[0]).unwrap();
        assert_eq!(
            *hint,
            FieldType::DateTime(DateTime::new(Some(vec![
                "%Y-%m-%d %H:%M:%S".to_string(),
                "%d/%b/%Y:%H:%M:%S".to_string()
            ])))
        );
        let re = regex::Regex::new(&expanded).unwrap();
        assert!(re.is_match("2023-05-03 12:34:56 - msg"));
        assert!(re.is_match("03/May/2023:12:34:56 - msg"));
    }

    #[test]
    fn test_custom_macro() {
        let custom = CustomMacro {
            name: "ip".to_string(),
            pattern: r"\d{1,3}(?:\.\d{1,3}){3}".to_string(),
            type_hint: Some(FieldType::String),
            description: Some("IPv4 address".to_string()),
        };
        let pat = "{{client:ip}} connected";
        let (expanded, fields, hints) = expand_macros(pat, Some(&[custom])).unwrap();
        assert!(expanded.contains(r"\d{1,3}(?:\.\d{1,3}){3}"));
        assert_eq!(fields, vec!["client".to_string()]);
        assert_eq!(*hints.get("client").unwrap(), FieldType::String);
    }

    #[test]
    fn test_datetime_macro_default() {
        let pat = "{{ts:datetime}} - msg";
        let (_expanded, fields, hints) = expand_macros(pat, None).unwrap();
        assert_eq!(fields.len(), 1);
        let hint = hints.get(&fields[0]).unwrap();
        assert_eq!(
            *hint,
            FieldType::DateTime(DateTime::new(Some(vec![
                "%Y-%m-%dT%H:%M:%S%.f".to_string()
            ])))
        );
    }

    #[test]
    fn test_float_macro() {
        let pat = "value: {{val:float}}";
        let (expanded, fields, hints) = expand_macros(pat, None).unwrap();

        let re = regex::Regex::new(&expanded).unwrap();

        // Integer compatible
        assert!(re.is_match("value: 123"));
        assert!(re.is_match("value: -5"));

        // Standard floats
        assert!(re.is_match("value: 123.456"));
        assert!(re.is_match("value: 0.1"));
        assert!(re.is_match("value: .5"));

        // Scientific notation
        assert!(re.is_match("value: 1.2e5"));
        assert!(re.is_match("value: 1E-10"));

        assert_eq!(fields, vec!["val".to_string()]);
        assert_eq!(*hints.get("val").unwrap(), FieldType::Float);
    }
}

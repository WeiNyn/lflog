//! Macro invocation parser.
//!
//! Parses macro invocations like `field:datetime("%Y-%m-%d")` into structured data.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::FieldType;

/// Represents a parsed macro invocation.
#[derive(Debug, Clone)]
pub struct MacroInvocation {
    pub field: Option<String>,
    pub name: String,
    pub args: Vec<String>,
}

/// Split a comma-separated argument string, respecting quoted strings.
pub fn split_args(s: &str) -> Vec<String> {
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
        } else if c == '\'' || c == '"' {
            in_quote = Some(c);
        } else if c == ',' {
            args.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
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

/// Parse a macro invocation string into a `MacroInvocation`.
///
/// Supports formats like:
/// - `field:macro_name(arg1, arg2)`
/// - `macro_name(arg1)`
/// - `field:macro_name`
/// - `macro_name`
pub fn parse_macro_invocation(s: &str) -> Result<MacroInvocation> {
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

#[derive(Serialize, Deserialize, Clone)]
pub struct CustomMacro {
    pub name: String,
    pub pattern: String,
    pub type_hint: Option<FieldType>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub custom_macros: Vec<CustomMacro>,
    pub pattern: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Profiles {
    #[serde(default)]
    pub custom_macros: Vec<CustomMacro>,
    pub profiles: Vec<Profile>,
}

impl Profiles {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut profiles: Profiles = toml::from_str(&content)?;

        // Merge profile macros with custom macros
        for profile in profiles.profiles.iter_mut() {
            profile.custom_macros.extend(profiles.custom_macros.clone());
        }

        Ok(profiles)
    }

    pub fn get_macro(&self, name: &str) -> Option<&CustomMacro> {
        self.custom_macros.iter().find(|m| m.name == name)
    }

    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }
}

impl Profile {
    pub fn get_macro(&self, name: &str) -> Option<&CustomMacro> {
        self.custom_macros.iter().find(|m| m.name == name)
    }
}

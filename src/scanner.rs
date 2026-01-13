//! Log line scanner using compiled regex patterns.

use regex::Regex;
use std::collections::HashMap;

use crate::macros::expand_macros;
use crate::macros::parser::CustomMacro;
use crate::types::FieldType;

/// Scans log lines using a compiled regex pattern with named capture groups.
#[derive(Debug, Clone)]
pub struct Scanner {
    regex: Regex,
    /// Ordered list of field names extracted from the pattern.
    pub field_names: Vec<String>,
    /// Type hints for fields, used for schema generation.
    pub type_hints: HashMap<String, FieldType>,
}

impl Scanner {
    /// Create a new Scanner from a pattern string using only builtin macros.
    ///
    /// The pattern can contain macros like `{{field:datetime("%Y-%m-%d")}}` or
    /// standard regex named capture groups like `(?P<name>...)`.
    pub fn new(pattern: String) -> Self {
        Self::with_custom_macros(pattern, None)
    }

    /// Create a new Scanner with custom macros.
    ///
    /// Custom macros are checked before builtin macros during expansion.
    pub fn with_custom_macros(pattern: String, custom_macros: Option<&[CustomMacro]>) -> Self {
        let (expanded, mut field_names, type_hints) =
            expand_macros(&pattern, custom_macros).unwrap();
        let regex = Regex::new(&expanded).unwrap();

        // If no macros were found, extract field names from regex named capture groups
        if field_names.is_empty() {
            field_names = regex
                .capture_names()
                .flatten()
                .map(|s| s.to_string())
                .collect();
        }

        Self {
            regex,
            field_names,
            type_hints,
        }
    }

    /// Scan a log line and return captured field values in order.
    ///
    /// Returns `None` if the line doesn't match the pattern.
    pub fn scan(&self, line: &str) -> Option<Vec<String>> {
        let caps = self.regex.captures(line)?;
        let out: Vec<String> = self
            .field_names
            .iter()
            .map(|n| {
                caps.name(n)
                    .map(|m| m.as_str().to_owned())
                    .unwrap_or_default()
            })
            .collect();
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_integration() {
        let pattern = r#"^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$"#;
        let scanner = Scanner::new(pattern.to_string());
        let line = "[Sun Dec 04 04:47:44 2005] [notice] workerEnv.init() ok /etc/httpd/conf/workers2.properties";
        let fields = scanner.scan(line).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "Sun Dec 04 04:47:44 2005");
        assert_eq!(fields[1], "notice");
    }
}

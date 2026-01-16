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
    /// Map of field names to their corresponding capture group indices.
    pub indices_map: HashMap<String, usize>,
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

        let indices_map = regex
            .capture_names()
            .enumerate()
            .filter_map(|(i, name_opt)| name_opt.map(|name| (name.to_string(), i)))
            .collect::<HashMap<String, usize>>();

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
            indices_map,
            field_names,
            type_hints,
        }
    }

    /// Prepare capture indices for a list of fields.
    ///
    /// This resolves the mapping from field names to regex capture group indices once,
    /// allowing for faster lookups during scanning.
    pub fn prepare_indices(&self, field_names: &[&str]) -> Vec<usize> {
        let mut indices = Vec::with_capacity(field_names.len());
        for name in field_names {
            if let Some(index) = self.indices_map.get(*name) {
                indices.push(*index);
            }
        }
        indices
    }

    /// Scan a log line using pre-calculated indices and populate a buffer with slices.
    ///
    /// This method avoids String allocations by returning slices of the input line (`&str`).
    /// It appends results to the provided `out` buffer, which should be reused across calls
    /// to minimize allocation overhead.
    ///
    /// Returns `true` if the line matches the pattern, `false` otherwise.
    pub fn scan_direct<'a>(
        &self,
        line: &'a str,
        field_indices: &[usize],
        out: &mut Vec<&'a str>,
    ) -> bool {
        if let Some(caps) = self.regex.captures(line) {
            out.clear();
            for &index in field_indices {
                out.push(caps.get(index).map(|m| m.as_str()).unwrap_or_default());
            }
            true
        } else {
            false
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

    /// Scan a log line and return captured values for specific fields only.
    ///
    /// Unlike [`scan`], this method returns only the values for the specified
    /// `field_names` in the order they are provided. This is useful for query
    /// projections where only a subset of fields are needed.
    ///
    /// Returns `None` if the line doesn't match the pattern.
    pub fn scan_with(&self, line: &str, field_names: &[&str]) -> Option<Vec<String>> {
        let caps = self.regex.captures(line)?;
        let out: Vec<String> = field_names
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

//! Log line scanner using compiled regex patterns.

use regex::Regex;
use std::collections::HashMap;

use crate::error::{Error, Result};
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
    pub fn new(pattern: String) -> Result<Self> {
        Self::with_custom_macros(pattern, None)
    }

    /// Create a new Scanner with custom macros.
    ///
    /// Custom macros are checked before builtin macros during expansion.
    pub fn with_custom_macros(
        pattern: String,
        custom_macros: Option<&[CustomMacro]>,
    ) -> Result<Self> {
        let (expanded, mut field_names, type_hints) = expand_macros(&pattern, custom_macros)?;
        let regex = Regex::new(&expanded)?;

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

        Ok(Self {
            regex,
            indices_map,
            field_names,
            type_hints,
        })
    }

    /// Prepare capture indices for a list of fields.
    ///
    /// This resolves the mapping from field names to regex capture group indices once,
    /// allowing for faster lookups during scanning.
    ///
    /// Returns an error if a field name is not found in either the regex capture groups
    /// or the additional columns.
    pub fn prepare_indices(
        &self,
        field_names: &[&str],
        additional_columns: &[&str],
    ) -> Result<Vec<usize>> {
        let capture_count = self.regex.captures_len();
        let additional_indices_map = additional_columns
            .iter()
            .enumerate()
            .map(|(i, name)| (*name, capture_count + i))
            .collect::<HashMap<&str, usize>>();

        let mut indices = Vec::with_capacity(field_names.len() + additional_columns.len());
        for name in field_names {
            if let Some(index) = self.indices_map.get(*name) {
                indices.push(*index);
            } else if let Some(index) = additional_indices_map.get(*name) {
                indices.push(*index);
            } else {
                return Err(Error::other(format!("Field name not found: {}", name)));
            }
        }
        Ok(indices)
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
        let scanner = Scanner::new(pattern.to_string()).unwrap();
        let line = "[Sun Dec 04 04:47:44 2005] [notice] workerEnv.init() ok /etc/httpd/conf/workers2.properties";
        let fields = scanner.scan(line).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "Sun Dec 04 04:47:44 2005");
        assert_eq!(fields[1], "notice");
    }

    #[test]
    fn test_scanner_no_named_groups_panic() {
        // Pattern with no named groups
        let pattern = r"^\d+$";
        let scanner = Scanner::new(pattern.to_string()).unwrap();

        // Field names should be empty since there are no named groups
        assert!(scanner.field_names.is_empty());

        // This panicked with the old logic: 0 + 0 - 1 (underflow)
        // New logic: captures_len() + 0
        // NOTE: We must request "__FILE__" in the field list for it to be indexed
        let indices = scanner
            .prepare_indices(&["__FILE__"], &["__FILE__"])
            .unwrap();

        assert_eq!(indices.len(), 1);
        // captures_len is 1 (group 0 is the whole match)
        assert!(indices[0] >= 1);
    }

    #[test]
    fn test_scanner_mixed_groups_indices() {
        // Pattern: Unnamed group (\d+) then named group (?P<name>\w+)
        // Captures: 0 (all), 1 (digits), 2 (name) -> captures_len = 3
        let pattern = r"^(\d+) (?P<name>\w+)$";
        let scanner = Scanner::new(pattern.to_string()).unwrap();

        assert_eq!(scanner.field_names, vec!["name"]);

        // We want 'name' and '__FILE__'
        // NOTE: We must request both in the field list
        let indices = scanner
            .prepare_indices(&["name", "__FILE__"], &["__FILE__"])
            .unwrap();

        assert_eq!(indices.len(), 2);

        // Index for 'name' should be 2
        assert_eq!(indices[0], 2);

        // Index for '__FILE__' should be >= 3 (captures_len)
        // Old logic: 0 + 1 (field_names.len) - 1 = 0 (Collision with group 0!)
        assert!(indices[1] >= 3);

        // Verify scan_direct behavior
        let line = "123 test";
        let mut values = Vec::new();
        let matched = scanner.scan_direct(line, &indices, &mut values);

        assert!(matched);
        assert_eq!(values[0], "test"); // name
        assert_eq!(values[1], ""); // __FILE__ (should be empty/None from regex)
    }
}

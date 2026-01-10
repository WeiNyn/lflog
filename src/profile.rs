use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Profile {
    name: String,
    prefilter: String,
    pattern: String,
    version: String,
    field_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Scanner {
    regex: Regex,
    pub field_names: Vec<String>,
}

impl Scanner {
    pub fn new(pattern: String) -> Self {
        let regex = Regex::new(&pattern).unwrap();
        let field_names = regex
            .capture_names()
            .filter_map(|s| s.map(|s| s.to_string()))
            .collect();
        Self { regex, field_names }
    }

    pub fn scan(&self, line: &str) -> Option<Vec<String>> {
        if let Some(captures) = self.regex.captures(line) {
            let mut values = Vec::new();
            for name in &self.field_names {
                if let Some(value) = captures.name(name) {
                    values.push(value.as_str().to_string());
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
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    use super::*;

    #[test]
    fn test_scanner() {
        let pattern = r"(?P<name>\w+) (?P<age>\d+)";
        let scanner = Scanner::new(pattern.to_string());
        let line = "Alice 30";
        let expected = Some(vec!["Alice".to_string(), "30".to_string()]);
        assert_eq!(scanner.scan(line), expected);
    }

    #[test]
    fn test_scanner_with_empty_line() {
        let pattern = r"(?P<name>\w+) (?P<age>\d+)";
        let scanner = Scanner::new(pattern.to_string());
        let line = "";
        let expected = None;
        assert_eq!(scanner.scan(line), expected);
    }

    #[test]
    #[should_panic(expected = "regex parse error")]
    fn test_scanner_with_invalid_pattern() {
        let pattern = r"(?P<name>\w+) (?P<age>\d+";
        let scanner = Scanner::new(pattern.to_string());
        let line = "Alice 30";
        let expected = None;
        assert_eq!(scanner.scan(line), expected);
    }

    #[test]
    fn test_scanner_with_invalid_line() {
        let pattern = r"(?P<name>\w+) (?P<age>\d+)";
        let scanner = Scanner::new(pattern.to_string());
        let line = "Alice";
        let expected = None;
        assert_eq!(scanner.scan(line), expected);
    }

    #[test]
    fn test_scanner_with_invalid_line_format() {
        let pattern = r"^(?P<name>\w+) (?P<age>\d+)$";
        let scanner = Scanner::new(pattern.to_string());
        let line = "Alice 30 40";
        let expected = None;
        assert_eq!(scanner.scan(line), expected);
    }

    #[test]
    fn test_scanner_with_real_log() {
        let pattern = r"^\[(?P<time>\w{3} \w{3} \d{1,2} \d{2}:\d{2}:\d{2} \d{4})\] \[(?P<level>[^\]]+)\] (?P<message>.*)$";
        let scanner = Scanner::new(pattern.to_string());
        let log_file = File::open("loghub/Apache/Apache_2k.log").unwrap();
        let reader = BufReader::new(log_file);
        for line_result in reader.lines() {
            let line = line_result.unwrap();
            if let Some(result) = scanner.scan(&line) {
                println!("{:?}", result);
            }
        }
    }
}

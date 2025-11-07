//! Utility functions for code generation.
//!
//! This module will contain helper functions used during code generation.
//! Implementation will be added during Phase 2 extraction.

use convert_case::{Case, Casing};

/// Convert a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}

/// Convert a string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

/// Convert a string to camelCase
pub fn to_camel_case(s: &str) -> String {
    s.to_case(Case::Camel)
}

/// Convert a string to SCREAMING_SNAKE_CASE
pub fn to_screaming_snake_case(s: &str) -> String {
    s.to_case(Case::ScreamingSnake)
}

/// Escape a string for use in Rust string literals
pub fn escape_rust_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Generate Rust type from field type string
pub fn rust_type_from_string(type_str: &str) -> String {
    match type_str {
        "String" => "String".to_string(),
        "Integer" => "i64".to_string(),
        "Float" => "f64".to_string(),
        "Boolean" => "bool".to_string(),
        "DateTime" => "String".to_string(), // Could use chrono::DateTime later
        "List[String]" => "Vec<String>".to_string(),
        _ => type_str.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_conversions() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_camel_case("hello_world"), "helloWorld");
        assert_eq!(to_screaming_snake_case("hello_world"), "HELLO_WORLD");
    }

    #[test]
    fn test_escape_rust_string() {
        assert_eq!(escape_rust_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_rust_string("say \"hello\""), "say \\\"hello\\\"");
    }

    #[test]
    fn test_rust_type_from_string() {
        assert_eq!(rust_type_from_string("String"), "String");
        assert_eq!(rust_type_from_string("Integer"), "i64");
        assert_eq!(rust_type_from_string("List[String]"), "Vec<String>");
    }
}

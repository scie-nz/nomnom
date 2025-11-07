//! Generic field extraction abstractions for structured data.
//!
//! This module provides format-agnostic patterns for extracting values from
//! structured data using path-based addressing.

use std::fmt;

/// Represents a path to a field in structured data
///
/// # Examples
///
/// - CSV: `column[0]` - First column
/// - JSON: `user.name` or `$.user.name` - JSONPath syntax
/// - XML: `//user/name` - XPath syntax
/// - Custom: Any delimiter-based path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPath {
    /// The raw path string
    pub raw: String,
    /// Parsed path segments
    pub segments: Vec<PathSegment>,
}

/// A segment in a field path
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// A named field (e.g., "user", "name")
    Field(String),
    /// An array index (e.g., [0], [5])
    Index(usize),
    /// A wildcard/glob pattern (e.g., "*", "**")
    Wildcard,
}

impl FieldPath {
    /// Parse a field path with a given delimiter
    ///
    /// # Example
    ///
    /// ```ignore
    /// use nomnom::FieldPath;
    ///
    /// let path = FieldPath::parse("user.address.city", ".");
    /// assert_eq!(path.segments.len(), 3);
    /// ```
    pub fn parse(path: &str, delimiter: &str) -> Self {
        let segments = path
            .split(delimiter)
            .filter(|s| !s.is_empty())
            .map(|s| {
                // Check if it's an array index
                if s.starts_with('[') && s.ends_with(']') {
                    if let Ok(index) = s[1..s.len() - 1].parse::<usize>() {
                        return PathSegment::Index(index);
                    }
                }

                // Check for wildcard
                if s == "*" || s == "**" {
                    return PathSegment::Wildcard;
                }

                // Otherwise it's a field name
                PathSegment::Field(s.to_string())
            })
            .collect();

        Self {
            raw: path.to_string(),
            segments,
        }
    }

    /// Create a field path from a dotted string (common format)
    pub fn from_dotted(path: &str) -> Self {
        Self::parse(path, ".")
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// Trait for types that can extract values by field path
///
/// This trait allows different data formats to implement their own
/// extraction logic while maintaining a common interface.
///
/// # Example
///
/// ```ignore
/// use nomnom::{Extractor, FieldPath};
///
/// struct CsvRow {
///     fields: Vec<String>,
/// }
///
/// impl Extractor for CsvRow {
///     fn extract(&self, path: &FieldPath) -> Option<String> {
///         // Extract by column index
///         if let Some(PathSegment::Index(idx)) = path.segments.first() {
///             self.fields.get(*idx).cloned()
///         } else {
///             None
///         }
///     }
/// }
/// ```
pub trait Extractor {
    /// Extract a value at the given field path
    ///
    /// Returns `Some(value)` if the path exists, `None` otherwise
    fn extract(&self, path: &FieldPath) -> Option<String>;

    /// Extract a value and parse it to a specific type
    fn extract_as<T>(&self, path: &FieldPath) -> Option<T>
    where
        T: std::str::FromStr,
    {
        self.extract(path).and_then(|s| s.parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_path_parse() {
        let path = FieldPath::parse("user.address.city", ".");

        assert_eq!(path.segments.len(), 3);
        assert_eq!(path.segments[0], PathSegment::Field("user".to_string()));
        assert_eq!(
            path.segments[1],
            PathSegment::Field("address".to_string())
        );
        assert_eq!(path.segments[2], PathSegment::Field("city".to_string()));
    }

    #[test]
    fn test_field_path_with_index() {
        let path = FieldPath::parse("items.[0].name", ".");

        assert_eq!(path.segments.len(), 3);
        assert_eq!(path.segments[0], PathSegment::Field("items".to_string()));
        assert_eq!(path.segments[1], PathSegment::Index(0));
        assert_eq!(path.segments[2], PathSegment::Field("name".to_string()));
    }

    #[test]
    fn test_field_path_from_dotted() {
        let path = FieldPath::from_dotted("a.b.c");

        assert_eq!(path.segments.len(), 3);
        assert_eq!(path.raw, "a.b.c");
    }

    struct SimpleExtractor {
        data: std::collections::HashMap<String, String>,
    }

    impl Extractor for SimpleExtractor {
        fn extract(&self, path: &FieldPath) -> Option<String> {
            if let Some(PathSegment::Field(name)) = path.segments.first() {
                self.data.get(name).cloned()
            } else {
                None
            }
        }
    }

    #[test]
    fn test_extractor() {
        let mut data = std::collections::HashMap::new();
        data.insert("name".to_string(), "Alice".to_string());
        data.insert("age".to_string(), "30".to_string());

        let extractor = SimpleExtractor { data };

        let name_path = FieldPath::from_dotted("name");
        assert_eq!(extractor.extract(&name_path), Some("Alice".to_string()));

        let age_path = FieldPath::from_dotted("age");
        assert_eq!(extractor.extract_as::<i32>(&age_path), Some(30));
    }
}

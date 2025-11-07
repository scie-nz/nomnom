//! Serialization framework for entities.
//!
//! This module provides utilities for serializing entities to various formats.

use serde::Serialize;
use std::io::Write;

/// Error type for serialization operations
#[derive(Debug)]
pub enum SerializationError {
    JsonError(serde_json::Error),
    IoError(std::io::Error),
}

impl From<serde_json::Error> for SerializationError {
    fn from(err: serde_json::Error) -> Self {
        SerializationError::JsonError(err)
    }
}

impl From<std::io::Error> for SerializationError {
    fn from(err: std::io::Error) -> Self {
        SerializationError::IoError(err)
    }
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationError::JsonError(e) => write!(f, "JSON error: {}", e),
            SerializationError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for SerializationError {}

/// NDJSON (Newline Delimited JSON) writer
///
/// Writes entities as NDJSON, one JSON object per line.
pub struct NdjsonWriter<W: Write> {
    writer: W,
}

impl<W: Write> NdjsonWriter<W> {
    /// Create a new NDJSON writer
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a single entity as an NDJSON line
    pub fn write<T: Serialize>(&mut self, entity: &T) -> Result<(), SerializationError> {
        let json = serde_json::to_string(entity)?;
        writeln!(self.writer, "{}", json)?;
        Ok(())
    }

    /// Write multiple entities
    pub fn write_all<T: Serialize>(
        &mut self,
        entities: &[T],
    ) -> Result<(), SerializationError> {
        for entity in entities {
            self.write(entity)?;
        }
        Ok(())
    }

    /// Flush the underlying writer
    pub fn flush(&mut self) -> Result<(), SerializationError> {
        self.writer.flush()?;
        Ok(())
    }
}

/// JSON array writer
///
/// Writes entities as a JSON array.
pub struct JsonArrayWriter<W: Write> {
    writer: W,
    first: bool,
}

impl<W: Write> JsonArrayWriter<W> {
    /// Create a new JSON array writer and write the opening bracket
    pub fn new(mut writer: W) -> Result<Self, SerializationError> {
        write!(writer, "[")?;
        Ok(Self {
            writer,
            first: true,
        })
    }

    /// Write a single entity to the JSON array
    pub fn write<T: Serialize>(&mut self, entity: &T) -> Result<(), SerializationError> {
        if !self.first {
            write!(self.writer, ",")?;
        }
        self.first = false;

        let json = serde_json::to_string(entity)?;
        write!(self.writer, "{}", json)?;
        Ok(())
    }

    /// Finish writing the array and close the bracket
    pub fn finish(mut self) -> Result<(), SerializationError> {
        write!(self.writer, "]")?;
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestEntity {
        name: String,
        value: i32,
    }

    #[test]
    fn test_ndjson_writer() {
        let mut buf = Vec::new();
        let mut writer = NdjsonWriter::new(&mut buf);

        let entity1 = TestEntity {
            name: "Alice".to_string(),
            value: 42,
        };
        let entity2 = TestEntity {
            name: "Bob".to_string(),
            value: 24,
        };

        writer.write(&entity1).unwrap();
        writer.write(&entity2).unwrap();
        writer.flush().unwrap();

        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Alice"));
        assert!(lines[1].contains("Bob"));
    }

    #[test]
    fn test_json_array_writer() {
        let mut buf = Vec::new();
        let mut writer = JsonArrayWriter::new(&mut buf).unwrap();

        let entity1 = TestEntity {
            name: "Alice".to_string(),
            value: 42,
        };
        let entity2 = TestEntity {
            name: "Bob".to_string(),
            value: 24,
        };

        writer.write(&entity1).unwrap();
        writer.write(&entity2).unwrap();
        writer.finish().unwrap();

        let output = String::from_utf8(buf).unwrap();

        assert!(output.starts_with('['));
        assert!(output.ends_with(']'));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
    }
}

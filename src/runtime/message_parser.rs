//! Generic message parser for entity extraction.
//!
//! Provides format-agnostic infrastructure for extracting entities from
//! structured messages in dependency order.

use std::collections::HashMap;
use serde_json::Value as JsonValue;
use crate::runtime::context::ExtractionContext;
use crate::runtime::config_loader::ParserConfig;

/// Trait for entity extraction from structured data.
///
/// Implement this trait to define how to extract an entity from a message
/// and its context. The extraction is format-agnostic - works with HL7v2,
/// JSON, CSV, XML, or any structured data.
pub trait EntityExtractor: Send + Sync {
    /// Extract entity from context.
    ///
    /// # Arguments
    /// * `context` - Extraction context with global fields and previously extracted entities
    /// * `message` - Raw message data (format-specific)
    ///
    /// # Returns
    /// * `Ok(Some(value))` - Entity extracted successfully
    /// * `Ok(None)` - Entity not found (optional entity)
    /// * `Err(msg)` - Extraction failed
    fn extract(
        &self,
        context: &ExtractionContext,
        message: &JsonValue,
    ) -> Result<Option<JsonValue>, String>;

    /// Get entity name.
    fn name(&self) -> &str;

    /// Check if this is a repeated entity (returns Vec instead of single value).
    fn is_repeated(&self) -> bool {
        false
    }
}

/// Generic message parser that extracts entities in dependency order.
///
/// Coordinates entity extraction using:
/// - Parser configuration (defines entities and extraction order)
/// - Entity extractors (implement extraction logic)
/// - Extraction context (holds extracted entities and global state)
///
/// # Example Flow
/// 1. Load parser config (defines entities: Filename, UserID, MPI, etc.)
/// 2. Register entity extractors (one per entity type)
/// 3. Parse message → extracts entities in topological order
/// 4. Results stored in ExtractionContext
pub struct MessageParser {
    /// Parser configuration (entities, dependencies, extraction order)
    config: ParserConfig,

    /// Entity extractors: entity_name -> extractor implementation
    extractors: HashMap<String, Box<dyn EntityExtractor>>,
}

impl MessageParser {
    /// Create a new message parser with configuration.
    ///
    /// # Arguments
    /// * `config` - Parser configuration defining entities and extraction order
    ///
    /// # Example
    /// ```ignore
    /// use nomnom::runtime::{MessageParser, ParserConfig};
    ///
    /// let config = ParserConfig::load_from_file("config/parser_config.yaml")?;
    /// let parser = MessageParser::new(config);
    /// ```
    pub fn new(config: ParserConfig) -> Self {
        Self {
            config,
            extractors: HashMap::new(),
        }
    }

    /// Register an entity extractor.
    ///
    /// # Arguments
    /// * `extractor` - Entity extractor implementation
    ///
    /// # Example
    /// ```ignore
    /// parser.register_extractor(Box::new(UserIDExtractor));
    /// parser.register_extractor(Box::new(MPIExtractor));
    /// ```
    pub fn register_extractor(&mut self, extractor: Box<dyn EntityExtractor>) {
        let name = extractor.name().to_string();
        self.extractors.insert(name, extractor);
    }

    /// Parse a message and extract all entities.
    ///
    /// Extracts entities in topological order (dependencies first).
    /// Results are stored in the returned ExtractionContext.
    ///
    /// # Arguments
    /// * `message` - Raw message data (format-specific, as JSON)
    /// * `global_context` - Global context fields (filename, source, timestamp, etc.)
    ///
    /// # Returns
    /// ExtractionContext with all extracted entities
    ///
    /// # Errors
    /// Returns error if extraction fails for a required entity
    ///
    /// # Example
    /// ```ignore
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let message = json!({
    ///     "MSH": "MSH|^~\\&|...",
    ///     "PID": "PID|1||123456^^^MRN||Smith^John||19800115|M"
    /// });
    ///
    /// let mut global_ctx = HashMap::new();
    /// global_ctx.insert("filename".to_string(), "test.csv".to_string());
    ///
    /// let context = parser.parse_message(&message, global_ctx)?;
    /// ```
    pub fn parse_message(
        &self,
        message: &JsonValue,
        global_context: HashMap<String, String>,
    ) -> Result<ExtractionContext, String> {
        let mut context = ExtractionContext::new(global_context);

        // Extract entities in topological order
        for entity_name in &self.config.extraction_order {
            // Get entity extractor
            let extractor = match self.extractors.get(entity_name) {
                Some(ext) => ext,
                None => {
                    // No extractor registered - skip this entity
                    continue;
                }
            };

            // Extract entity
            match extractor.extract(&context, message) {
                Ok(Some(value)) => {
                    // Entity extracted successfully
                    context.set_entity(entity_name.clone(), value);
                }
                Ok(None) => {
                    // Entity not found (optional entity)
                    continue;
                }
                Err(err) => {
                    // Extraction failed - check if entity is required
                    if self.is_required_entity(entity_name) {
                        return Err(format!(
                            "Failed to extract required entity '{}': {}",
                            entity_name, err
                        ));
                    }
                    // Optional entity - continue
                }
            }
        }

        Ok(context)
    }

    /// Parse a message with an existing context.
    ///
    /// Useful for incremental parsing or when you need to provide
    /// pre-extracted entities.
    ///
    /// # Arguments
    /// * `message` - Raw message data
    /// * `context` - Existing extraction context (will be modified)
    ///
    /// # Returns
    /// Unit result (context is modified in-place)
    pub fn parse_message_with_context(
        &self,
        message: &JsonValue,
        context: &mut ExtractionContext,
    ) -> Result<(), String> {
        // Extract entities in topological order
        for entity_name in &self.config.extraction_order {
            // Skip if already extracted
            if context.has_entity(entity_name) {
                continue;
            }

            // Get entity extractor
            let extractor = match self.extractors.get(entity_name) {
                Some(ext) => ext,
                None => continue,
            };

            // Extract entity
            match extractor.extract(context, message) {
                Ok(Some(value)) => {
                    context.set_entity(entity_name.clone(), value);
                }
                Ok(None) => continue,
                Err(err) => {
                    if self.is_required_entity(entity_name) {
                        return Err(format!(
                            "Failed to extract required entity '{}': {}",
                            entity_name, err
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract a single entity by name.
    ///
    /// Useful for extracting specific entities without full message parsing.
    /// Note: Dependencies must already be in context!
    ///
    /// # Arguments
    /// * `entity_name` - Name of entity to extract
    /// * `message` - Raw message data
    /// * `context` - Extraction context (dependencies must already be present)
    ///
    /// # Returns
    /// Extracted entity value or None
    pub fn extract_entity(
        &self,
        entity_name: &str,
        message: &JsonValue,
        context: &ExtractionContext,
    ) -> Result<Option<JsonValue>, String> {
        let extractor = self.extractors.get(entity_name).ok_or_else(|| {
            format!("No extractor registered for entity '{}'", entity_name)
        })?;

        extractor.extract(context, message)
    }

    /// Check if an entity is required (extraction failure is an error).
    ///
    /// # Arguments
    /// * `entity_name` - Entity name
    ///
    /// # Returns
    /// true if entity is required
    fn is_required_entity(&self, entity_name: &str) -> bool {
        self.config
            .entities
            .get(entity_name)
            .and_then(|e| e.get("required"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Get entity configuration.
    ///
    /// # Arguments
    /// * `entity_name` - Entity name
    ///
    /// # Returns
    /// Entity configuration or None
    pub fn get_entity_config(&self, entity_name: &str) -> Option<&HashMap<String, JsonValue>> {
        self.config.entities.get(entity_name)
    }

    /// Get extraction order (topologically sorted entity names).
    pub fn get_extraction_order(&self) -> &[String] {
        &self.config.extraction_order
    }

    /// Get all entity names in the configuration.
    pub fn get_entity_names(&self) -> Vec<&String> {
        self.config.entities.keys().collect()
    }

    /// Get registered extractor names.
    pub fn get_registered_extractors(&self) -> Vec<&String> {
        self.extractors.keys().collect()
    }

    /// Check if an extractor is registered for an entity.
    pub fn has_extractor(&self, entity_name: &str) -> bool {
        self.extractors.contains_key(entity_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Mock entity extractor for testing
    struct MockExtractor {
        name: String,
        result: Option<JsonValue>,
    }

    impl EntityExtractor for MockExtractor {
        fn extract(
            &self,
            _context: &ExtractionContext,
            _message: &JsonValue,
        ) -> Result<Option<JsonValue>, String> {
            Ok(self.result.clone())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_create_message_parser() {
        let config = ParserConfig {
            entities: HashMap::new(),
            extraction_order: vec![],
        };

        let parser = MessageParser::new(config);
        assert_eq!(parser.get_extraction_order().len(), 0);
        assert_eq!(parser.get_entity_names().len(), 0);
    }

    #[test]
    fn test_register_extractor() {
        let config = ParserConfig {
            entities: HashMap::new(),
            extraction_order: vec![],
        };

        let mut parser = MessageParser::new(config);

        let extractor = MockExtractor {
            name: "TestEntity".to_string(),
            result: Some(json!({"field": "value"})),
        };

        parser.register_extractor(Box::new(extractor));

        assert!(parser.has_extractor("TestEntity"));
        assert!(!parser.has_extractor("MissingEntity"));
    }

    #[test]
    fn test_parse_message_simple() {
        let mut entities = HashMap::new();
        entities.insert("Entity1".to_string(), HashMap::new());
        entities.insert("Entity2".to_string(), HashMap::new());

        let config = ParserConfig {
            entities,
            extraction_order: vec!["Entity1".to_string(), "Entity2".to_string()],
        };

        let mut parser = MessageParser::new(config);

        // Register extractors
        parser.register_extractor(Box::new(MockExtractor {
            name: "Entity1".to_string(),
            result: Some(json!({"name": "first"})),
        }));

        parser.register_extractor(Box::new(MockExtractor {
            name: "Entity2".to_string(),
            result: Some(json!({"name": "second"})),
        }));

        let message = json!({});
        let mut global_ctx = HashMap::new();
        global_ctx.insert("filename".to_string(), "test.txt".to_string());

        let context = parser.parse_message(&message, global_ctx).unwrap();

        assert!(context.has_entity("Entity1"));
        assert!(context.has_entity("Entity2"));
        assert_eq!(
            context.get_entity("Entity1"),
            Some(&json!({"name": "first"}))
        );
        assert_eq!(
            context.get_entity("Entity2"),
            Some(&json!({"name": "second"}))
        );
    }

    #[test]
    fn test_parse_message_optional_entity_missing() {
        let mut entities = HashMap::new();
        entities.insert("Entity1".to_string(), HashMap::new());

        let config = ParserConfig {
            entities,
            extraction_order: vec!["Entity1".to_string()],
        };

        let mut parser = MessageParser::new(config);

        // Register extractor that returns None (entity not found)
        parser.register_extractor(Box::new(MockExtractor {
            name: "Entity1".to_string(),
            result: None,
        }));

        let message = json!({});
        let global_ctx = HashMap::new();

        let context = parser.parse_message(&message, global_ctx).unwrap();

        // Optional entity missing - should not fail
        assert!(!context.has_entity("Entity1"));
    }

    #[test]
    fn test_extract_single_entity() {
        let mut entities = HashMap::new();
        entities.insert("TestEntity".to_string(), HashMap::new());

        let config = ParserConfig {
            entities,
            extraction_order: vec!["TestEntity".to_string()],
        };

        let mut parser = MessageParser::new(config);

        parser.register_extractor(Box::new(MockExtractor {
            name: "TestEntity".to_string(),
            result: Some(json!({"value": 123})),
        }));

        let message = json!({});
        let context = ExtractionContext::empty();

        let result = parser
            .extract_entity("TestEntity", &message, &context)
            .unwrap();

        assert_eq!(result, Some(json!({"value": 123})));
    }

    #[test]
    fn test_extract_entity_no_extractor() {
        let config = ParserConfig {
            entities: HashMap::new(),
            extraction_order: vec![],
        };

        let parser = MessageParser::new(config);
        let message = json!({});
        let context = ExtractionContext::empty();

        let result = parser.extract_entity("MissingEntity", &message, &context);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("No extractor registered for entity"));
    }
}

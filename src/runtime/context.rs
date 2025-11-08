//! Generic extraction context for entity extraction.
//!
//! Provides a format-agnostic context that holds extracted entities and global
//! metadata during message parsing.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Context for entity extraction from a single message.
///
/// Holds extracted entities and global context metadata during parsing.
/// Generic over entity types - works with any structured data format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionContext {
    /// Extracted entities: entity_name -> serialized entity data
    /// For singleton entities: maps to single value
    /// For repeated entities: maps to Vec of values
    #[serde(default)]
    extracted: HashMap<String, serde_json::Value>,

    /// Global context fields available to all entities
    #[serde(default)]
    global_context: HashMap<String, String>,
}

impl ExtractionContext {
    /// Create a new extraction context with initial global context.
    ///
    /// # Arguments
    /// * `global_context` - Global context fields (e.g., filename, source, timestamp)
    ///
    /// # Example
    /// ```
    /// use std::collections::HashMap;
    /// use nomnom::runtime::ExtractionContext;
    ///
    /// let mut context_fields = HashMap::new();
    /// context_fields.insert("filename".to_string(), "test.csv".to_string());
    /// context_fields.insert("source".to_string(), "hospital_a".to_string());
    ///
    /// let ctx = ExtractionContext::new(context_fields);
    /// ```
    pub fn new(global_context: HashMap<String, String>) -> Self {
        Self {
            extracted: HashMap::new(),
            global_context,
        }
    }

    /// Create an empty extraction context.
    pub fn empty() -> Self {
        Self {
            extracted: HashMap::new(),
            global_context: HashMap::new(),
        }
    }

    /// Get a previously extracted entity by name.
    ///
    /// # Arguments
    /// * `name` - Entity name
    ///
    /// # Returns
    /// `Some(value)` if entity exists, `None` otherwise
    ///
    /// # Example
    /// ```
    /// # use nomnom::runtime::ExtractionContext;
    /// let ctx = ExtractionContext::empty();
    /// let entity = ctx.get_entity("UserIdentification");
    /// ```
    pub fn get_entity(&self, name: &str) -> Option<&serde_json::Value> {
        self.extracted.get(name)
    }

    /// Store an extracted entity.
    ///
    /// # Arguments
    /// * `name` - Entity name
    /// * `value` - Entity data (singleton or Vec for repeated)
    ///
    /// # Example
    /// ```
    /// # use nomnom::runtime::ExtractionContext;
    /// # use serde_json::json;
    /// let mut ctx = ExtractionContext::empty();
    /// ctx.set_entity("Profile".to_string(), json!({"lastname": "Smith", "firstname": "John"}));
    /// ```
    pub fn set_entity(&mut self, name: String, value: serde_json::Value) {
        self.extracted.insert(name, value);
    }

    /// Check if an entity has been extracted.
    ///
    /// # Arguments
    /// * `name` - Entity name
    ///
    /// # Returns
    /// `true` if entity exists in extracted map
    pub fn has_entity(&self, name: &str) -> bool {
        self.extracted.contains_key(name)
    }

    /// Get global context field by name.
    ///
    /// # Arguments
    /// * `key` - Context field name
    ///
    /// # Returns
    /// `Some(&value)` if field exists, `None` otherwise
    pub fn get_context_field(&self, key: &str) -> Option<&String> {
        self.global_context.get(key)
    }

    /// Set a global context field.
    ///
    /// # Arguments
    /// * `key` - Context field name
    /// * `value` - Context field value
    pub fn set_context_field(&mut self, key: String, value: String) {
        self.global_context.insert(key, value);
    }

    /// Get all global context fields as a HashMap.
    ///
    /// # Returns
    /// Reference to global context fields
    pub fn get_context_dict(&self) -> &HashMap<String, String> {
        &self.global_context
    }

    /// Get all extracted entities.
    ///
    /// # Returns
    /// Reference to extracted entities map
    pub fn get_extracted(&self) -> &HashMap<String, serde_json::Value> {
        &self.extracted
    }

    /// Get mutable reference to extracted entities (for advanced use).
    pub fn get_extracted_mut(&mut self) -> &mut HashMap<String, serde_json::Value> {
        &mut self.extracted
    }

    /// Convert context to JSON for serialization.
    ///
    /// # Returns
    /// JSON representation of the entire context
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Create context from JSON.
    ///
    /// # Arguments
    /// * `json` - JSON representation of context
    ///
    /// # Returns
    /// Deserialized ExtractionContext
    pub fn from_json(json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(json.clone())
    }

    /// Get the number of extracted entities.
    pub fn entity_count(&self) -> usize {
        self.extracted.len()
    }

    /// Clear all extracted entities (keeps global context).
    pub fn clear_extracted(&mut self) {
        self.extracted.clear();
    }

    /// Get all entity names that have been extracted.
    ///
    /// # Returns
    /// Iterator over entity names
    pub fn entity_names(&self) -> impl Iterator<Item = &String> {
        self.extracted.keys()
    }
}

impl Default for ExtractionContext {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_empty_context() {
        let ctx = ExtractionContext::empty();
        assert_eq!(ctx.entity_count(), 0);
        assert_eq!(ctx.get_context_dict().len(), 0);
    }

    #[test]
    fn test_create_with_global_context() {
        let mut global_ctx = HashMap::new();
        global_ctx.insert("filename".to_string(), "test.csv".to_string());
        global_ctx.insert("source".to_string(), "hospital_a".to_string());

        let ctx = ExtractionContext::new(global_ctx);

        assert_eq!(ctx.get_context_field("filename"), Some(&"test.csv".to_string()));
        assert_eq!(ctx.get_context_field("source"), Some(&"hospital_a".to_string()));
        assert_eq!(ctx.get_context_field("missing"), None);
    }

    #[test]
    fn test_set_and_get_entity() {
        let mut ctx = ExtractionContext::empty();

        let entity_data = json!({
            "lastname": "Smith",
            "firstname": "John",
            "dob": "19800115"
        });

        ctx.set_entity("Profile".to_string(), entity_data.clone());

        assert!(ctx.has_entity("Profile"));
        assert!(!ctx.has_entity("Location"));
        assert_eq!(ctx.get_entity("Profile"), Some(&entity_data));
        assert_eq!(ctx.entity_count(), 1);
    }

    #[test]
    fn test_set_repeated_entity() {
        let mut ctx = ExtractionContext::empty();

        let procedures = json!([
            {"procedure_code": "12345", "procedure_text": "Surgery A"},
            {"procedure_code": "67890", "procedure_text": "Surgery B"}
        ]);

        ctx.set_entity("Action".to_string(), procedures.clone());

        assert!(ctx.has_entity("Action"));
        assert_eq!(ctx.get_entity("Action"), Some(&procedures));
    }

    #[test]
    fn test_entity_names() {
        let mut ctx = ExtractionContext::empty();

        ctx.set_entity("Profile".to_string(), json!({"lastname": "Smith"}));
        ctx.set_entity("Location".to_string(), json!({"code": "ABC"}));
        ctx.set_entity("Action".to_string(), json!([]));

        let names: Vec<&String> = ctx.entity_names().collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&&"Profile".to_string()));
        assert!(names.contains(&&"Location".to_string()));
        assert!(names.contains(&&"Action".to_string()));
    }

    #[test]
    fn test_clear_extracted() {
        let mut ctx = ExtractionContext::empty();
        ctx.set_entity("Profile".to_string(), json!({"lastname": "Smith"}));
        ctx.set_entity("Location".to_string(), json!({"code": "ABC"}));

        assert_eq!(ctx.entity_count(), 2);

        ctx.clear_extracted();

        assert_eq!(ctx.entity_count(), 0);
        assert!(!ctx.has_entity("Profile"));
        assert!(!ctx.has_entity("Location"));
    }

    #[test]
    fn test_set_context_field() {
        let mut ctx = ExtractionContext::empty();

        ctx.set_context_field("filename".to_string(), "test.csv".to_string());
        ctx.set_context_field("timestamp".to_string(), "20250128120000".to_string());

        assert_eq!(ctx.get_context_field("filename"), Some(&"test.csv".to_string()));
        assert_eq!(ctx.get_context_field("timestamp"), Some(&"20250128120000".to_string()));
    }

    #[test]
    fn test_serialization() {
        let mut global_ctx = HashMap::new();
        global_ctx.insert("filename".to_string(), "test.csv".to_string());

        let mut ctx = ExtractionContext::new(global_ctx);
        ctx.set_entity("Profile".to_string(), json!({"lastname": "Smith"}));

        // Serialize to JSON
        let json = ctx.to_json().expect("Should serialize");

        // Deserialize back
        let ctx2 = ExtractionContext::from_json(&json).expect("Should deserialize");

        assert_eq!(ctx2.get_context_field("filename"), Some(&"test.csv".to_string()));
        assert_eq!(ctx2.get_entity("Profile"), Some(&json!({"lastname": "Smith"})));
    }
}

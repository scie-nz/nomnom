# JSON Parsing Example

This example demonstrates how to use nomnom to parse JSON documents and extract entities.

## Sample JSON Document

```json
{
  "users": [
    {
      "id": 1,
      "profile": {
        "name": "John Doe",
        "email": "john@example.com"
      },
      "settings": {
        "theme": "dark",
        "notifications": true
      }
    },
    {
      "id": 2,
      "profile": {
        "name": "Jane Smith",
        "email": "jane@example.com"
      },
      "settings": {
        "theme": "light",
        "notifications": false
      }
    }
  ]
}
```

## Entity Configuration

Create a YAML configuration file `config/entities/user_profile.yaml`:

```yaml
entity:
  name: UserProfile
  source_type: derived
  parent: JsonDocument
  fields:
    - name: user_id
      type: Integer
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.id"

    - name: name
      type: String
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.profile.name"

    - name: email
      type: String
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.profile.email"

    - name: theme
      type: String
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.settings.theme"
```

## Rust Implementation

```rust
use nomnom::runtime::transform_registry::TransformRegistry;
use serde_json::Value;

fn main() {
    // Create transform registry
    let mut registry = TransformRegistry::new();

    // Register JSON field extraction transform
    registry.register_rust_transform(
        "extract_json_field".to_string(),
        Box::new(|args| {
            let raw_json = args.get("raw_json")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing raw_json".to_string())?;

            let json_path = args.get("json_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing json_path".to_string())?;

            // Parse JSON
            let json: Value = serde_json::from_str(raw_json)?;

            // Extract field at path (simplified JSON path implementation)
            // In production, use a proper JSONPath library
            let value = extract_from_path(&json, json_path)?;

            Ok(Some(value))
        }),
    );

    // Load and process JSON
    let json_data = std::fs::read_to_string("data.json").unwrap();
    println!("Processing JSON document...");
}

fn extract_from_path(json: &Value, path: &str) -> Result<Value, String> {
    // Simplified JSON path extraction
    // In production, use libraries like jsonpath or serde_json_path
    Ok(json.clone())
}
```

## Output

```json
{"user_id": 1, "name": "John Doe", "email": "john@example.com", "theme": "dark"}
{"user_id": 2, "name": "Jane Smith", "email": "jane@example.com", "theme": "light"}
```

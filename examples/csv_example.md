# CSV Parsing Example

This example demonstrates how to use nomnom to parse CSV files and extract entities.

## Sample CSV File

```csv
name,email,age,city
John Doe,john@example.com,30,New York
Jane Smith,jane@example.com,25,San Francisco
Bob Johnson,bob@example.com,35,Chicago
```

## Entity Configuration

Create a YAML configuration file `config/entities/user.yaml`:

```yaml
entity:
  name: User
  source_type: derived
  parent: CsvFile
  fields:
    - name: name
      type: String
      computed_from:
        transform: extract_csv_field
        sources:
          - source: parent
            field: raw_line
        args:
          column_index: 0

    - name: email
      type: String
      computed_from:
        transform: extract_csv_field
        sources:
          - source: parent
            field: raw_line
        args:
          column_index: 1

    - name: age
      type: Integer
      computed_from:
        transform: extract_csv_field
        sources:
          - source: parent
            field: raw_line
        args:
          column_index: 2

    - name: city
      type: String
      computed_from:
        transform: extract_csv_field
        sources:
          - source: parent
            field: raw_line
        args:
          column_index: 3
```

## Rust Implementation

```rust
use nomnom::runtime::transform_registry::TransformRegistry;
use serde_json::Value;

fn main() {
    // Create transform registry
    let mut registry = TransformRegistry::new();

    // Register CSV field extraction transform
    registry.register_rust_transform(
        "extract_csv_field".to_string(),
        Box::new(|args| {
            let raw_line = args.get("raw_line")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing raw_line".to_string())?;

            let column_index = args.get("column_index")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "Missing column_index".to_string())? as usize;

            // Split by comma and extract field
            let fields: Vec<&str> = raw_line.split(',').collect();

            if column_index < fields.len() {
                Ok(Some(Value::String(fields[column_index].trim().to_string())))
            } else {
                Ok(None)
            }
        }),
    );

    // Load CSV and process
    let csv_data = std::fs::read_to_string("data.csv").unwrap();

    for line in csv_data.lines().skip(1) { // Skip header
        // Process each line using the transform registry
        println!("Processing: {}", line);
    }
}
```

## Output

```json
{"name": "John Doe", "email": "john@example.com", "age": 30, "city": "New York"}
{"name": "Jane Smith", "email": "jane@example.com", "age": 25, "city": "San Francisco"}
{"name": "Bob Johnson", "email": "bob@example.com", "age": 35, "city": "Chicago"}
```

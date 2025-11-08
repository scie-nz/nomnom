# Transform YAML Schema

This document defines the YAML schema for transforms in the nomnom framework.

## Overview

Transforms are functions that compute field values from source data. They can be:
- **Built-in transforms**: Implemented in Rust (e.g., `extract_field`, `parse_date`)
- **Custom transforms**: Inline code in YAML (Rust or Python)
- **Transform chains**: Transforms that call other transforms

## Transform YAML Structure

### Location

Transforms are defined in two places:

1. **config/transforms/** - Standalone transform definitions
2. **Entity YAMLs** - Inline in field `computed_from` blocks

### Basic Transform Definition

```yaml
transform:
  name: extract_field
  language: rust  # or 'python'
  doc: "Extract value from structured data using path notation"

  parameters:
    - name: data
      type: String
      doc: "Raw data string"

    - name: field_path
      type: String
      doc: "Path notation (e.g., 'field.subfield.index')"

  returns:
    type: Option<String>
    doc: "Extracted value or None if not present"

  implementation:
    type: builtin  # or 'inline', 'reference'
    # For builtin: no code needed (implemented in Rust)
    # For inline: provide code block
    # For reference: call another transform
```

### Inline Transform (in Entity YAML)

```yaml
fields:
  - name: category_code
    type: String
    nullable: true
    computed_from:
      transform: extract_field
      sources:
        - data  # Variable name from context
      args:
        field_path: category.code
```

### Inline Code Transform

For format-specific logic not yet in built-in transforms:

```yaml
transform:
  name: parse_custom_date
  language: rust
  doc: "Parse custom date format"

  parameters:
    - name: date_string
      type: String

  returns:
    type: Option<String>

  implementation:
    type: inline
    code: |
      // Rust code here
      if date_string.is_empty() {
          return None;
      }

      // Parse YYYYMMDD to YYYY-MM-DD
      if date_string.len() == 8 {
          let year = &date_string[0..4];
          let month = &date_string[4..6];
          let day = &date_string[6..8];
          Some(format!("{}-{}-{}", year, month, day))
      } else {
          None
      }
```

### Python Inline Transform

For transforms that need Python runtime (slower but more flexible):

```yaml
transform:
  name: compute_identifier
  language: python
  doc: "Compute identifier from multiple sources with fallback logic"

  parameters:
    - name: record
      type: Record
    - name: default_value
      type: String

  returns:
    type: String

  implementation:
    type: inline
    code: |
      # Python code here
      if record.primary_id:
          return record.primary_id.strip()
      elif record.secondary_id:
          return record.secondary_id.strip()
      else:
          return default_value
```

### Transform Chain (Reference)

Call another transform from a transform:

```yaml
transform:
  name: extract_and_normalize_date
  language: rust
  doc: "Extract date from data and normalize format"

  parameters:
    - name: data
      type: String
    - name: field_path
      type: String

  returns:
    type: Option<String>

  implementation:
    type: reference
    steps:
      - transform: extract_field
        args:
          data: $data
          field_path: $field_path
        output: raw_date

      - transform: parse_custom_date
        args:
          date_string: $raw_date
        output: normalized_date

    return: $normalized_date
```

## Built-in Transforms

### Generic Transforms

#### copy_from_parent

Copy field value from parent entity.

```yaml
computed_from:
  transform: copy_from_parent
  sources:
    - parent
  args:
    field_name: "user_id"
```

#### copy_from_context

Copy value from global context.

```yaml
computed_from:
  transform: copy_from_context
  args:
    context_field: "filename"
```

#### coalesce

Return first non-null value from sources.

```yaml
computed_from:
  transform: coalesce
  sources:
    - field1
    - field2
    - field3
```

## Transform Registry

Transforms are registered at runtime in the TransformRegistry:

```rust
use nomnom::TransformRegistry;

let mut registry = TransformRegistry::new();

// Register built-in transforms
registry.register_builtin("extract_field", extract_field);
registry.register_builtin("parse_date", parse_date);

// Load custom transforms from YAML
registry.load_transforms_from_dir("config/transforms/")?;

// Use in entity extraction
let value = registry.call("extract_field", args)?;
```

## Code Generation

The codegen system generates:

1. **Rust code** for inline Rust transforms
2. **Python bindings** via PyO3 for calling Rust transforms from Python
3. **Python code** for inline Python transforms

### Generated Rust Transform

```rust
// Generated from transform YAML
pub fn parse_custom_date(date_string: &str) -> Option<String> {
    // Inline code from YAML
    if date_string.is_empty() {
        return None;
    }

    if date_string.len() == 8 {
        let year = &date_string[0..4];
        let month = &date_string[4..6];
        let day = &date_string[6..8];
        Some(format!("{}-{}-{}", year, month, day))
    } else {
        None
    }
}
```

### Generated Python Binding

```python
# Auto-generated via PyO3
from data_processor._rust import parse_custom_date

# Can be called from Python
result = parse_custom_date("20250128")
# Returns: "2025-01-28"
```

## Validation

Transform YAMLs are validated at load time:

- ✅ Required fields present (name, language, parameters, returns, implementation)
- ✅ Parameter types valid
- ✅ Return type valid
- ✅ Referenced transforms exist
- ✅ Inline code compiles (at build time)
- ✅ No circular dependencies in transform chains

## Testing

Transform YAMLs can include unit tests:

```yaml
transform:
  name: parse_custom_date
  # ... transform definition ...

  tests:
    - name: test_valid_date
      args:
        date_string: "20250128"
      expected: "2025-01-28"

    - name: test_empty_string
      args:
        date_string: ""
      expected: null

    - name: test_invalid_format
      args:
        date_string: "2025-01-28"
      expected: null
```

Tests are auto-generated as Rust unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_date() {
        assert_eq!(
            parse_custom_date("20250128"),
            Some("2025-01-28".to_string())
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(parse_custom_date(""), None);
    }
}
```

## Best Practices

When creating transforms:

1. **Start with inline Python** for rapid prototyping
2. **Convert to Rust** for performance-critical transforms
3. **Add tests** to ensure correctness
4. **Document parameters** clearly
5. **Use descriptive names** that indicate the transform's purpose

Example progression:

**Stage 1: Inline Python (rapid prototyping)**
```yaml
transform:
  name: compute_identifier
  language: python
  implementation:
    type: inline
    code: |
      if record.primary_id:
          return record.primary_id.strip()
      return default_value
```

**Stage 2: Add tests and documentation**
```yaml
transform:
  name: compute_identifier
  language: python
  doc: "Compute identifier with fallback logic"

  parameters:
    - name: record
      type: Record
    - name: default_value
      type: String
      default: ""

  returns:
    type: String

  implementation:
    type: inline
    code: |
      if record.primary_id:
          return record.primary_id.strip()
      return default_value

  tests:
    - name: test_with_primary_id
      args:
        record: {primary_id: "ID123"}
        default_value: "DEFAULT"
      expected: "ID123"
```

**Stage 3: Port to Rust (optional)**
```rust
// For performance-critical code
pub fn compute_identifier(record: &Record, default_value: &str) -> String {
    record.primary_id
        .as_ref()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| default_value.to_string())
}
```

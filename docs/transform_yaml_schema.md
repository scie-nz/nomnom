# Transform YAML Schema

This document defines the YAML schema for transforms in the nomnom framework.

## Overview

Transforms are functions that compute field values from source data. They can be:
- **Built-in transforms**: Implemented in Rust (e.g., `extract_from_hl7_segment`)
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
  name: extract_from_hl7_segment
  language: rust  # or 'python'
  doc: "Extract value from HL7 segment using path notation"

  parameters:
    - name: segment
      type: String
      doc: "Raw HL7 segment string"

    - name: segment_path
      type: String
      doc: "Path notation (e.g., 'DG1.3.1' for DG1-3.1)"

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
      transform: extract_from_hl7_segment
      sources:
        - segment  # Variable name from context
      args:
        segment_path: DG1.3.1
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
  name: compute_accid
  language: python
  doc: "Compute account ID from multiple sources with fallback logic"

  parameters:
    - name: user_account
      type: UserAccount
    - name: default_value
      type: String

  returns:
    type: String

  implementation:
    type: inline
    code: |
      # Python code here
      if user_account.user_account_number:
          return user_account.user_account_number.strip()
      elif user_account.visit_number:
          return user_account.visit_number.strip()
      else:
          return default_value
```

### Transform Chain (Reference)

Call another transform from a transform:

```yaml
transform:
  name: extract_and_normalize_date
  language: rust
  doc: "Extract date from segment and normalize format"

  parameters:
    - name: segment
      type: String
    - name: segment_path
      type: String

  returns:
    type: Option<String>

  implementation:
    type: reference
    steps:
      - transform: extract_from_hl7_segment
        args:
          segment: $segment
          segment_path: $segment_path
        output: raw_date

      - transform: parse_custom_date
        args:
          date_string: $raw_date
        output: normalized_date

    return: $normalized_date
```

## Built-in Transforms

### structured data-Specific Transforms

#### extract_from_hl7_segment

Extract value from HL7 segment using path notation.

```yaml
computed_from:
  transform: extract_from_hl7_segment
  sources:
    - segment
  args:
    segment_path: "DG1.3.1"  # Extract DG1-3.1
```

**Parameters:**
- `segment: String` - Raw HL7 segment string
- `segment_path: String` - Path notation (e.g., "DG1.3.1")

**Returns:** `Option<String>`

**Path Format:**
- `SEGMENT.field` - Extract field (1-based)
- `SEGMENT.field.component` - Extract component (1-based)
- `SEGMENT.field.component.subcomponent` - Extract subcomponent (1-based)

#### build_segment_index

Build segment index from raw structured data message.

```yaml
computed_from:
  transform: build_segment_index
  sources:
    - raw_message
```

**Parameters:**
- `raw_message: String` - Raw structured data message

**Returns:** `HashMap<String, Vec<String>>` - Maps segment type to list of segments

#### extract_msh_field

Extract field from MSH segment (special numbering).

```yaml
computed_from:
  transform: extract_msh_field
  sources:
    - raw_message
  args:
    field_index: 9  # MSH-9
```

**Parameters:**
- `raw_message: String` - Raw structured data message
- `field_index: usize` - Field number (1-based)

**Returns:** `Option<String>`

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
registry.register_builtin("extract_from_hl7_segment", extract_from_hl7_segment);
registry.register_builtin("build_segment_index", build_segment_index);

// Load custom transforms from YAML
registry.load_transforms_from_dir("config/transforms/")?;

// Use in entity extraction
let value = registry.call("extract_from_hl7_segment", args)?;
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

## Migration Path

Existing transforms (Python functions) will be migrated to YAML in phases:

1. **Extract current Python transforms** from `extraction_functions.py`
2. **Convert to inline Python transforms** in YAML (no behavior change)
3. **Port to Rust** for performance (optional, can stay Python)
4. **Delete Python source files** once YAMLs complete

Example migration:

**Before (Python):**
```python
# extraction_functions.py
def compute_accid(user_account, default_value=""):
    if user_account.user_account_number:
        return user_account.user_account_number.strip()
    return default_value
```

**After (YAML):**
```yaml
# config/transforms/compute_accid.yaml
transform:
  name: compute_accid
  language: python
  doc: "Compute account ID with fallback logic"

  parameters:
    - name: user_account
      type: UserAccount
    - name: default_value
      type: String
      default: ""

  returns:
    type: String

  implementation:
    type: inline
    code: |
      if user_account.user_account_number:
          return user_account.user_account_number.strip()
      return default_value

  tests:
    - name: test_with_account_number
      args:
        user_account: {user_account_number: "ACC123"}
        default_value: "DEFAULT"
      expected: "ACC123"
```

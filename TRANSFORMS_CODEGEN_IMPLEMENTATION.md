# Transform Function Codegen Implementation

## Summary

Successfully implemented automatic generation of custom transform functions from `nomnom.yaml` into worker binaries. The framework now reads transform definitions from the project's configuration file and generates Rust code with proper function signatures, imports, and implementations.

## Changes Made

### 1. Transform Loading (`src/bin/nomnom.rs`)

Added logic to load `nomnom.yaml` when generating workers:

```rust
// Try to load nomnom.yaml for transforms (optional)
// Look for nomnom.yaml in: entities parent dir, current dir, and entities_dir itself
let nomnom_yaml = nomnom_yaml_candidates.iter()
    .find(|path| path.exists())
    .cloned();

let transforms = if let Some(nomnom_yaml_path) = nomnom_yaml {
    match nomnom::codegen::project_config::BuildConfig::from_file(&nomnom_yaml_path) {
        Ok(config) => {
            println!("  ✓ Loaded {} custom transforms", transform_count);
            config.transforms.map(|t| t.rust)
        }
        // ... error handling ...
    }
} else {
    None
};
```

**Search Strategy**:
1. Parent directory of entities dir (e.g., `entities/../nomnom.yaml`)
2. Current working directory (`./nomnom.yaml`)
3. Inside entities directory (`entities/nomnom.yaml`)

### 2. Transform Generation (`src/codegen/worker/transforms_rs.rs`)

Completely rewrote to generate transforms from YAML instead of hardcoded defaults:

```rust
pub fn generate_transforms_rs(
    output_dir: &Path,
    transforms: Option<&HashMap<String, RustTransformDef>>,
) -> Result<(), Box<dyn Error>> {
    // Generate custom transforms from nomnom.yaml
    if let Some(transforms) = transforms {
        generate_custom_transforms(&mut output, transforms)?;
    } else {
        writeln!(output, "// No custom transforms defined in nomnom.yaml")?;
    }
    Ok(())
}
```

**Generated Code Structure**:
1. Collect and generate all unique imports from transform definitions
2. Generate each transform function with:
   - Documentation comments
   - Function signature (name, arguments with types)
   - Return type
   - Function body (indented properly)

### 3. Dependencies (`src/codegen/worker/cargo_toml.rs`)

Added common transform utilities to generated `Cargo.toml`:

```toml
# Transform utilities
regex = "1"
once_cell = "1"
```

These are commonly needed for HL7 parsing, filename extraction, etc.

### 4. Worker Module Signature (`src/codegen/worker/mod.rs`)

Updated `generate_all()` to accept transforms parameter:

```rust
pub fn generate_all(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &WorkerConfig,
    transforms: Option<&HashMap<String, RustTransformDef>>,
) -> Result<(), Box<dyn Error>>
```

### 5. Field Extraction Bug Fixes (`src/codegen/worker/main_rs.rs`)

Fixed handling of `Result<Option<T>>` return types from transforms:

**Before** (caused double-wrapping):
```rust
let field = transform_func(args).ok(); // Result<Option<T>> -> Option<Option<T>>
```

**After** (correct unwrapping):
```rust
let field = transform_func(args).unwrap_or(None); // Result<Option<T>> -> Option<T>
```

**Type Annotations for TODOs**:
```rust
// Before (type inference error)
let field = None; // TODO: ...

// After (explicit type)
let field: Option<String> = None; // TODO: ...
```

## Example: HL7 Project Transforms

From `hl7-nomnom-parser/nomnom.yaml`:

```yaml
transforms:
  rust:
    extract_filename_component:
      doc: "Extract component from filename using regex pattern"
      imports:
        - "regex::Regex"
      args:
        - name: filename
          type: "&str"
        - name: component
          type: "&str"
      return_type: "Result<Option<String>, String>"
      code: |
        let pattern = match component {
            "facilityId" => r"([A-Z]+)",
            "date" => r"(\d{8})",
            _ => return Ok(None),
        };

        let re = Regex::new(pattern)
            .map_err(|e| format!("Invalid regex pattern: {}", e))?;

        Ok(re.captures(filename)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string()))
```

**Generated Code** (`worker/src/transforms.rs`):

```rust
use regex::Regex;

/// Extract component from filename using regex pattern
pub fn extract_filename_component(filename: &str, component: &str) -> Result<Option<String>, String> {
    let pattern = match component {
        "facilityId" => r"([A-Z]+)",
        "date" => r"(\d{8})",
        _ => return Ok(None),
    };

    let re = Regex::new(pattern)
        .map_err(|e| format!("Invalid regex pattern: {}", e))?;

    Ok(re.captures(filename)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string()))
}
```

## Current Status

### ✅ Working
- Transform loading from `nomnom.yaml`
- Transform function generation with imports, signatures, and bodies
- Field extraction from root entities (e.g., `Filename` from `Hl7v2MessageFile`)
- Worker compiles successfully
- Database insertion logic for all persistent entities

### ⚠️ Remaining Work

**Intermediate Entity Instantiation from Segments**

Entities like `PatientVisit` and `PatientIdentification` derive from HL7 segments (PV1, PID) which need to be extracted from the raw HL7 message field. Current generated code has TODOs:

```rust
// Needs: Hl7v2MessageFile.hl7v2_message field
let patientvisit_patient_class: Option<String> = None; // TODO: Transform from Hl7v2Message -> PV1
let patientidentification_patient_identifier: Option<String> = None; // TODO: Transform from Hl7v2Message -> PID
```

**Required Enhancements**:
1. Ensure `Hl7v2MessageFile` entity exposes `hl7v2_message` field in parsers
2. Generate segment extraction calls: `extract_segment(&hl7v2message, "PV1", 0)?`
3. Generate intermediate entity instantiation from extracted segments
4. Handle segment-to-entity field mappings (e.g., `PV1.3.1` -> `attending_doctor_id`)

**Example of What's Needed**:
```rust
// Extract PV1 segment from HL7 message
let pv1_segment = extract_segment(&hl7v2messagefile.hl7v2_message, "PV1", 0).unwrap_or(None);

if let Some(pv1) = pv1_segment {
    // Extract fields from PV1 segment
    let patientvisit_attending_doctor_id = extract_from_hl7_segment(&pv1, "PV1.7.1").unwrap_or(None);
    let patientvisit_attending_doctor_last_name = extract_from_hl7_segment(&pv1, "PV1.7.2").unwrap_or(None);
    // ... etc
}
```

## Testing

### Verification Steps

```bash
# 1. Build nomnom
cd ~/claude-code/nomnom
cargo build

# 2. Generate worker
cd ~/claude-code/ingestion/hl7-nomnom-parser
~/claude-code/nomnom/target/debug/nomnom generate-worker \
  --entities entities \
  --output worker \
  --database postgresql

# Expected output:
# ✓ Loaded 16 entities
# ✓ Loaded 7 custom transforms

# 3. Build worker
cd worker
cargo build

# Expected: Successful build with warnings only
```

### Current Build Status

```
Compiling worker v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.49s
```

✅ Worker builds successfully with 7 generated transforms

## Benefits

1. **DRY Principle**: Define transforms once in `nomnom.yaml`, use across all generated code
2. **Type Safety**: Transform signatures are generated with proper Rust types
3. **Domain Flexibility**: Projects can define domain-specific transforms (HL7, EDI, CSV, etc.)
4. **No Hardcoding**: Removed all default/generic transform helpers from framework
5. **Easy Maintenance**: Update transforms in YAML, regenerate workers
6. **Reusability**: Same transforms used in parsers, field extraction, validation, etc.

## Files Modified

### Nomnom Framework
- `src/bin/nomnom.rs` - Load nomnom.yaml and pass transforms to worker generator
- `src/codegen/worker/mod.rs` - Accept transforms parameter in generate_all()
- `src/codegen/worker/transforms_rs.rs` - Generate custom transforms from YAML
- `src/codegen/worker/cargo_toml.rs` - Add regex and once_cell dependencies
- `src/codegen/worker/main_rs.rs` - Fix Result<Option<T>> handling

### Generated Output (hl7-nomnom-parser/worker/)
- `src/transforms.rs` - 7 custom transforms generated from YAML
- `src/main.rs` - Field extraction using generated transforms (partial)
- `Cargo.toml` - Includes regex and once_cell dependencies

## Commits

1. **4e310ee**: Implement intermediate entity instantiation and field extraction
2. **dae0be5**: Add basic field extraction from root entities
3. **fbe6d20**: Generalize worker codegen to support non-root persistent entities
4. **68cb4fd**: Add transform function codegen from nomnom.yaml ← **This implementation**

## Next Steps

To complete end-to-end field extraction for HL7 system:

1. **Expose hl7v2_message field** in Hl7v2MessageFile message struct
2. **Implement segment extraction** in generate_derived_entity_extraction()
3. **Generate intermediate entity instantiation** from extracted segments
4. **Map segment paths to fields** using entity YAML configurations
5. **Test with real HL7 messages** to verify complete pipeline

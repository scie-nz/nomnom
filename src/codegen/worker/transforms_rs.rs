/// Generate transforms.rs with helper functions for derived entity field extraction

use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_transforms_rs(
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let transforms_file = output_dir.join("src/transforms.rs");
    let mut output = std::fs::File::create(&transforms_file)?;

    writeln!(output, "// Auto-generated transform helper functions")?;
    writeln!(output, "// Generated for derived entity field extraction\n")?;

    writeln!(output, "use crate::error::AppError;")?;
    writeln!(output, "use serde_json::Value;\n")?;

    // Generate JSON extraction functions for required fields
    writeln!(output, "/// Extract required string field from JSON object")?;
    writeln!(output, "pub fn json_get_string(obj: &Value, field: &str) -> Result<String, AppError> {{")?;
    writeln!(output, "    obj.get(field)")?;
    writeln!(output, "        .and_then(|v| v.as_str())")?;
    writeln!(output, "        .map(|s| s.to_string())")?;
    writeln!(output, "        .ok_or_else(|| AppError::InvalidField(field.to_string()))")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Extract required integer field from JSON object")?;
    writeln!(output, "pub fn json_get_int(obj: &Value, field: &str) -> Result<i32, AppError> {{")?;
    writeln!(output, "    obj.get(field)")?;
    writeln!(output, "        .and_then(|v| v.as_i64())")?;
    writeln!(output, "        .map(|x| x as i32)")?;
    writeln!(output, "        .ok_or_else(|| AppError::InvalidField(field.to_string()))")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Extract required float field from JSON object")?;
    writeln!(output, "pub fn json_get_float(obj: &Value, field: &str) -> Result<f64, AppError> {{")?;
    writeln!(output, "    obj.get(field)")?;
    writeln!(output, "        .and_then(|v| v.as_f64())")?;
    writeln!(output, "        .ok_or_else(|| AppError::InvalidField(field.to_string()))")?;
    writeln!(output, "}}\n")?;

    // Generate JSON extraction functions for optional fields
    writeln!(output, "/// Extract optional string field from JSON object")?;
    writeln!(output, "pub fn json_get_optional_string(obj: &Value, field: &str) -> Option<String> {{")?;
    writeln!(output, "    obj.get(field)")?;
    writeln!(output, "        .and_then(|v| if v.is_null() {{ None }} else {{ v.as_str().map(|s| s.to_string()) }})")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Extract optional integer field from JSON object")?;
    writeln!(output, "pub fn json_get_optional_int(obj: &Value, field: &str) -> Option<i32> {{")?;
    writeln!(output, "    obj.get(field)")?;
    writeln!(output, "        .and_then(|v| if v.is_null() {{ None }} else {{ v.as_i64().map(|x| x as i32) }})")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Extract optional float field from JSON object")?;
    writeln!(output, "pub fn json_get_optional_float(obj: &Value, field: &str) -> Option<f64> {{")?;
    writeln!(output, "    obj.get(field)")?;
    writeln!(output, "        .and_then(|v| if v.is_null() {{ None }} else {{ v.as_f64() }})")?;
    writeln!(output, "}}\n")?;

    Ok(())
}

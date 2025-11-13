/// Generate models.rs for request/response types

use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_models_rs(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let models_file = output_dir.join("src/models.rs");
    let mut output = std::fs::File::create(&models_file)?;

    writeln!(output, "// Auto-generated request/response models")?;
    writeln!(output)?;
    writeln!(output, "use serde::{{Serialize, Deserialize}};")?;
    writeln!(output, "use utoipa::ToSchema;")?;
    writeln!(output, "use chrono::{{DateTime, Utc}};\n")?;

    // IngestRequest
    writeln!(output, "/// Request for single message ingestion")?;
    writeln!(output, "#[derive(Debug, Serialize, Deserialize, ToSchema)]")?;
    writeln!(output, "pub struct IngestRequest {{")?;
    writeln!(output, "    /// Base64-encoded JSON message body")?;
    writeln!(output, "    pub body_base64: String,")?;
    writeln!(output, "    /// Optional entity type hint")?;
    writeln!(output, "    pub entity_type: Option<String>,")?;
    writeln!(output, "}}\n")?;

    // IngestResponse
    writeln!(output, "/// Response from single message ingestion")?;
    writeln!(output, "#[derive(Debug, Serialize, Deserialize, ToSchema)]")?;
    writeln!(output, "pub struct IngestResponse {{")?;
    writeln!(output, "    pub status: String,")?;
    writeln!(output, "    pub entity: String,")?;
    writeln!(output, "    pub id: i64,")?;
    writeln!(output, "    pub timestamp: DateTime<Utc>,")?;
    writeln!(output, "    pub duration_ms: u64,")?;
    writeln!(output, "}}\n")?;

    // BatchResponse
    writeln!(output, "/// Response from batch ingestion")?;
    writeln!(output, "#[derive(Debug, Serialize, Deserialize, ToSchema)]")?;
    writeln!(output, "pub struct BatchResponse {{")?;
    writeln!(output, "    pub status: String,")?;
    writeln!(output, "    pub processed: usize,")?;
    writeln!(output, "    pub inserted: usize,")?;
    writeln!(output, "    pub failed: usize,")?;
    writeln!(output, "    pub errors: Vec<String>,")?;
    writeln!(output, "    pub duration_ms: u64,")?;
    writeln!(output, "}}\n")?;

    // HealthResponse
    writeln!(output, "/// Health check response")?;
    writeln!(output, "#[derive(Debug, Serialize, Deserialize, ToSchema)]")?;
    writeln!(output, "pub struct HealthResponse {{")?;
    writeln!(output, "    pub status: String,")?;
    writeln!(output, "    pub database: String,")?;
    writeln!(output, "    pub entities: Vec<String>,")?;
    writeln!(output, "    pub version: String,")?;
    writeln!(output, "}}\n")?;

    // StatsResponse
    writeln!(output, "/// Statistics response")?;
    writeln!(output, "#[derive(Debug, Serialize, Deserialize, ToSchema)]")?;
    writeln!(output, "pub struct StatsResponse {{")?;
    writeln!(output, "    pub total_messages_processed: u64,")?;
    writeln!(output, "    pub messages_by_entity: std::collections::HashMap<String, u64>,")?;
    writeln!(output, "    pub errors: u64,")?;
    writeln!(output, "    pub uptime_seconds: u64,")?;
    writeln!(output, "}}\n")?;

    Ok(())
}

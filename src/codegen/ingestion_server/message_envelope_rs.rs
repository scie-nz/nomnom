/// Generate message_envelope.rs for NATS message wrapping

use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_message_envelope_rs(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let file_path = output_dir.join("src/message_envelope.rs");
    let mut file = std::fs::File::create(&file_path)?;

    writeln!(file, "/// Message envelope for NATS JetStream")?;
    writeln!(file, "///")?;
    writeln!(file, "/// Wraps raw message body with metadata for tracking and processing")?;
    writeln!(file)?;
    writeln!(file, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(file, "use uuid::Uuid;")?;
    writeln!(file, "use chrono::{{DateTime, Utc}};")?;
    writeln!(file)?;
    writeln!(file, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
    writeln!(file, "pub struct MessageEnvelope {{")?;
    writeln!(file, "    /// Unique message ID for tracking")?;
    writeln!(file, "    pub message_id: Uuid,")?;
    writeln!(file)?;
    writeln!(file, "    /// Raw message body (before parsing)")?;
    writeln!(file, "    pub body: String,")?;
    writeln!(file)?;
    writeln!(file, "    /// Entity type hint (from URL path or header)")?;
    writeln!(file, "    pub entity_type: Option<String>,")?;
    writeln!(file)?;
    writeln!(file, "    /// Timestamp when message was received")?;
    writeln!(file, "    pub received_at: DateTime<Utc>,")?;
    writeln!(file)?;
    writeln!(file, "    /// Retry count")?;
    writeln!(file, "    #[serde(default)]")?;
    writeln!(file, "    pub retry_count: u32,")?;
    writeln!(file)?;
    writeln!(file, "    /// Source IP or identifier")?;
    writeln!(file, "    pub source: Option<String>,")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, "impl MessageEnvelope {{")?;
    writeln!(file, "    /// Create a new message envelope")?;
    writeln!(file, "    pub fn new(body: String, entity_type: Option<String>) -> Self {{")?;
    writeln!(file, "        Self {{")?;
    writeln!(file, "            message_id: Uuid::new_v4(),")?;
    writeln!(file, "            body,")?;
    writeln!(file, "            entity_type,")?;
    writeln!(file, "            received_at: Utc::now(),")?;
    writeln!(file, "            retry_count: 0,")?;
    writeln!(file, "            source: None,")?;
    writeln!(file, "        }}")?;
    writeln!(file, "    }}")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, "/// Response returned to client after ingestion")?;
    writeln!(file, "#[derive(Debug, Serialize)]")?;
    writeln!(file, "pub struct IngestionResponse {{")?;
    writeln!(file, "    pub message_id: String,")?;
    writeln!(file, "    pub status: IngestionStatus,")?;
    writeln!(file, "    pub timestamp: DateTime<Utc>,")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, "#[derive(Debug, Serialize)]")?;
    writeln!(file, "#[serde(rename_all = \"lowercase\")]")?;
    writeln!(file, "pub enum IngestionStatus {{")?;
    writeln!(file, "    Accepted,   // Queued in NATS")?;
    writeln!(file, "    Persisted,  // Written to DB (for sync mode)")?;
    writeln!(file, "    Failed,     // Validation or other error")?;
    writeln!(file, "}}")?;

    Ok(())
}

/// Message envelope for NATS JetStream
///
/// Wraps raw message body with metadata for tracking and processing

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Unique message ID for tracking
    pub message_id: Uuid,

    /// Raw message body (before parsing)
    pub body: String,

    /// Entity type hint (from URL path or header)
    pub entity_type: Option<String>,

    /// Timestamp when message was received
    pub received_at: DateTime<Utc>,

    /// Retry count
    #[serde(default)]
    pub retry_count: u32,

    /// Source IP or identifier
    pub source: Option<String>,
}

impl MessageEnvelope {
    /// Create a new message envelope
    pub fn new(body: String, entity_type: Option<String>) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            body,
            entity_type,
            received_at: Utc::now(),
            retry_count: 0,
            source: None,
        }
    }
}

/// Response returned to client after ingestion
#[derive(Debug, Serialize)]
pub struct IngestionResponse {
    pub message_id: String,
    pub status: IngestionStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IngestionStatus {
    Accepted,   // Queued in NATS
    Persisted,  // Written to DB (for sync mode)
    Failed,     // Validation or other error
}

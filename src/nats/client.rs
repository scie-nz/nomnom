/// NATS JetStream client for message publishing
///
/// Provides connection management and message publishing to NATS JetStream

use async_nats::jetstream;
use std::time::Duration;
use crate::nats::message_envelope::MessageEnvelope;

#[derive(Clone)]
pub struct NatsConfig {
    pub url: String,
    pub stream_name: String,
    pub max_age: Duration,
    pub max_bytes: i64,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("NATS_URL")
                .unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            stream_name: std::env::var("NATS_STREAM")
                .unwrap_or_else(|_| "MESSAGES".to_string()),
            max_age: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_bytes: 1024 * 1024 * 1024, // 1GB
        }
    }
}

#[derive(Clone)]
pub struct NatsClient {
    client: async_nats::Client,
    jetstream: jetstream::Context,
    stream_name: String,
}

impl NatsClient {
    /// Connect to NATS and initialize JetStream
    pub async fn connect(config: NatsConfig) -> Result<Self, async_nats::Error> {
        // Connect to NATS
        let client = async_nats::connect(&config.url).await?;
        tracing::info!("Connected to NATS at {}", config.url);

        // Get JetStream context
        let jetstream = jetstream::new(client.clone());

        // Create or get stream
        let _stream = jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: config.stream_name.clone(),
                subjects: vec!["messages.>".to_string()],
                max_age: config.max_age,
                max_bytes: config.max_bytes,
                storage: jetstream::stream::StorageType::File,
                num_replicas: 1,
                ..Default::default()
            })
            .await?;

        tracing::info!("JetStream stream '{}' ready", config.stream_name);

        Ok(Self {
            client,
            jetstream,
            stream_name: config.stream_name,
        })
    }

    /// Publish a message to JetStream
    pub async fn publish_message(
        &self,
        envelope: &MessageEnvelope,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let subject = format!("messages.ingest.{}",
            envelope.entity_type.as_deref().unwrap_or("default"));

        let payload = serde_json::to_vec(envelope)?;

        // Publish with JetStream (durable, acknowledged)
        let ack = self.jetstream
            .publish(subject.clone(), payload.into())
            .await?;

        // Wait for acknowledgment
        ack.await?;

        tracing::debug!(
            "Published message {} to JetStream subject {}",
            envelope.message_id,
            subject
        );

        Ok(())
    }

    /// Get JetStream context for advanced operations
    pub fn jetstream(&self) -> &jetstream::Context {
        &self.jetstream
    }

    /// Check if the NATS connection is active
    pub fn is_connected(&self) -> bool {
        self.client.connection_state() == async_nats::connection::State::Connected
    }
}

/// NATS JetStream integration module
///
/// Provides generic message envelope and NATS client for async message processing

pub mod message_envelope;
pub mod client;

pub use message_envelope::{MessageEnvelope, IngestionResponse, IngestionStatus};
pub use client::{NatsClient, NatsConfig};

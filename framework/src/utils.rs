//! Utility functions for the Solace Protocol

use crate::types::Timestamp;

/// Generate a unique identifier string
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Check if a timestamp is within a duration from now
pub fn is_within_duration(timestamp: Timestamp, duration_seconds: i64) -> bool {
    let now = Timestamp::now();
    let diff = now.0.timestamp() - timestamp.0.timestamp();
    diff.abs() <= duration_seconds
}

/// Format a timestamp for display
pub fn format_timestamp(timestamp: Timestamp) -> String {
    timestamp.0.format("%Y-%m-%d %H:%M:%S UTC").to_string()
} 
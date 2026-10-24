pub mod backup;
pub mod connection;
pub mod migrations;
pub mod search_index;

pub use connection::Database;

use std::time::{SystemTime, UNIX_EPOCH};

/// All timestamps in the store are Unix milliseconds UTC.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

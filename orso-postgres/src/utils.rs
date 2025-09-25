//! Utility functions for ORSO

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::OrsoDateTime;

/// Utility functions for ORSO
#[derive(Debug, Clone)]
pub struct Utils;

impl Utils {
    pub fn generate_id() -> Option<String> {
        Some(Uuid::new_v4().to_string())
    }

    pub fn current_timestamp() -> Option<OrsoDateTime> {
        Some(OrsoDateTime::now())
    }

    pub fn create_timestamp(timestamp: OrsoDateTime) -> String {
        timestamp.inner().to_rfc3339()
    }

    pub fn parse_timestamp(timestamp: &str) -> Result<OrsoDateTime, chrono::ParseError> {
        if timestamp.is_empty() {
            // Create a ParseError for empty input - use a dummy parse to get the error type
            return "".parse::<DateTime<Utc>>().map(OrsoDateTime::new).map_err(|e| e);
        }
        DateTime::parse_from_rfc3339(timestamp)
            .map(|dt| OrsoDateTime::new(dt.with_timezone(&Utc)))
    }

    /// Convert OrsoDateTime to Unix timestamp (seconds since epoch)
    pub fn datetime_to_unix_timestamp(dt: &OrsoDateTime) -> i64 {
        dt.inner().timestamp()
    }

    /// Convert Unix timestamp (seconds since epoch) to OrsoDateTime
    pub fn unix_timestamp_to_datetime(timestamp: i64) -> OrsoDateTime {
        let dt = DateTime::from_timestamp(timestamp, 0).unwrap_or_else(|| Utc::now());
        OrsoDateTime::new(dt)
    }

    /// Convert OrsoDateTime to Unix timestamp with milliseconds
    pub fn datetime_to_unix_timestamp_millis(dt: &OrsoDateTime) -> i64 {
        dt.inner().timestamp_millis()
    }

    /// Convert Unix timestamp with milliseconds to OrsoDateTime
    pub fn unix_timestamp_millis_to_datetime(timestamp: i64) -> OrsoDateTime {
        let dt = DateTime::from_timestamp_millis(timestamp).unwrap_or_else(|| Utc::now());
        OrsoDateTime::new(dt)
    }

    /// Convert our Value type to PostgreSQL parameter
    pub fn value_to_postgres_param(value: &crate::Value) -> Box<dyn tokio_postgres::types::ToSql + Send + Sync> {
        match value {
            crate::Value::Null => Box::new(Option::<String>::None),
            crate::Value::Integer(i) => {
                // Check if the value fits in i32 range for PostgreSQL INTEGER columns
                if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                    // Use i32 to ensure compatibility with INTEGER columns
                    Box::new(*i as i32)
                } else {
                    // Use i64 for BIGINT columns
                    Box::new(*i)
                }
            },
            crate::Value::Real(f) => Box::new(*f),
            crate::Value::Text(s) => Box::new(s.clone()),
            crate::Value::Blob(b) => Box::new(b.clone()),
            crate::Value::Boolean(b) => Box::new(*b),
            crate::Value::DateTime(dt) => Box::new(std::time::SystemTime::from(*dt.inner())),
            crate::Value::IntegerArray(arr) => Box::new(arr.clone()),
            crate::Value::BigIntArray(arr) => Box::new(arr.clone()),
            crate::Value::NumericArray(arr) => Box::new(arr.clone()),
        }
    }

    /// Convert PostgreSQL row value to our Value type
    pub fn postgres_row_to_value(row: &tokio_postgres::Row, idx: usize) -> crate::Result<crate::Value> {
        crate::Value::from_postgres_row(row, idx)
    }
}

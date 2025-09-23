//! Utility functions for ORSO

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Utility functions for ORSO
#[derive(Debug, Clone)]
pub struct Utils;

impl Utils {
    pub fn generate_id() -> Option<String> {
        Some(Uuid::new_v4().to_string())
    }

    pub fn current_timestamp() -> Option<DateTime<Utc>> {
        Some(Utc::now())
    }

    pub fn create_timestamp(timestamp: DateTime<Utc>) -> String {
        timestamp.to_rfc3339()
    }

    pub fn parse_timestamp(timestamp: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
        DateTime::parse_from_rfc3339(timestamp).map(|dt| dt.with_timezone(&Utc))
    }

    /// Convert DateTime to Unix timestamp (seconds since epoch)
    pub fn datetime_to_unix_timestamp(dt: &DateTime<Utc>) -> i64 {
        dt.timestamp()
    }

    /// Convert Unix timestamp (seconds since epoch) to DateTime
    pub fn unix_timestamp_to_datetime(timestamp: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(timestamp, 0).unwrap_or_else(|| Utc::now())
    }

    /// Convert DateTime to Unix timestamp with milliseconds
    pub fn datetime_to_unix_timestamp_millis(dt: &DateTime<Utc>) -> i64 {
        dt.timestamp_millis()
    }

    /// Convert Unix timestamp with milliseconds to DateTime
    pub fn unix_timestamp_millis_to_datetime(timestamp: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(timestamp).unwrap_or_else(|| Utc::now())
    }

    /// Convert our Value type to PostgreSQL parameter
    pub fn value_to_postgres_param(value: &crate::Value) -> Box<dyn tokio_postgres::types::ToSql + Send + Sync> {
        match value {
            crate::Value::Null => Box::new(Option::<String>::None),
            crate::Value::Integer(i) => Box::new(*i),
            crate::Value::Real(f) => Box::new(*f),
            crate::Value::Text(s) => Box::new(s.clone()),
            crate::Value::Blob(b) => Box::new(b.clone()),
            crate::Value::Boolean(b) => Box::new(*b),
        }
    }

    /// Convert PostgreSQL row value to our Value type
    pub fn postgres_row_to_value(row: &tokio_postgres::Row, idx: usize) -> crate::Result<crate::Value> {
        crate::Value::from_postgres_row(row, idx)
    }
}

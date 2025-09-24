// Unified error handling system for orso-postgres

use thiserror::Error;

/// Comprehensive error type for all orso-postgres operations
#[derive(Error, Debug)]
pub enum Error {
    // === Database Layer Errors ===
    /// Database connection errors (pool, network, auth)
    #[error("Database connection error: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// PostgreSQL query execution errors
    #[error("PostgreSQL error: {message}")]
    PostgreSql {
        message: String,
        code: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Connection pool errors (timeout, exhausted, etc.)
    #[error("Connection pool error: {message}")]
    Pool {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // === Query Building Errors ===
    /// SQL query building and parsing errors
    #[error("Query error: {message}")]
    Query {
        message: String,
        query: Option<String>,
        context: Option<String>,
    },

    /// Filter and condition building errors
    #[error("Filter error: {message}")]
    Filter {
        message: String,
        filter_type: Option<String>,
    },

    /// Pagination parameter errors
    #[error("Pagination error: {message}")]
    Pagination {
        message: String,
        page: Option<u32>,
        per_page: Option<u32>,
    },

    // === Data Handling Errors ===
    /// JSON serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        field: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Data validation errors (constraints, formats, etc.)
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
        value: Option<String>,
    },

    /// Type conversion errors
    #[error("Type conversion error: {message}")]
    TypeConversion {
        message: String,
        from_type: String,
        to_type: String,
    },

    // === Schema & Migration Errors ===
    /// Database schema migration errors
    #[error("Migration error: {message}")]
    Migration {
        message: String,
        table: Option<String>,
        operation: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Schema definition and validation errors
    #[error("Schema error: {message}")]
    Schema {
        message: String,
        table: Option<String>,
        column: Option<String>,
    },

    // === Configuration Errors ===
    /// Database connection configuration errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        parameter: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // === CRUD Operation Errors ===
    /// Record not found errors
    #[error("Record not found: {message}")]
    NotFound {
        message: String,
        table: Option<String>,
        key: Option<String>,
    },

    /// CRUD operation errors (insert, update, delete)
    #[error("Operation error: {message}")]
    Operation {
        message: String,
        operation: String,
        table: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Constraint violation errors (unique, foreign key, etc.)
    #[error("Constraint violation: {message}")]
    Constraint {
        message: String,
        constraint_type: Option<String>,
        table: Option<String>,
        column: Option<String>,
    },

    // === Compression Errors ===
    /// Data compression/decompression errors
    #[error("Compression error: {message}")]
    Compression {
        message: String,
        algorithm: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // === DateTime Errors ===
    /// DateTime parsing and handling errors
    #[error("DateTime error: {message}")]
    DateTime {
        message: String,
        input: Option<String>,
        format: Option<String>,
    },

    // === System Errors ===
    /// IO and file system errors
    #[error("IO error: {message}")]
    Io {
        message: String,
        operation: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Internal system errors
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        location: Option<String>,
    },
}

// === Error Construction Helper Methods ===
impl Error {
    /// Create a connection error with context
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Create a connection error with source
    pub fn connection_with_source(message: impl Into<String>, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Connection {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a PostgreSQL error with optional code
    pub fn postgres(message: impl Into<String>, code: Option<String>) -> Self {
        Self::PostgreSql {
            message: message.into(),
            code,
            source: None,
        }
    }

    /// Create a query error with context
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query {
            message: message.into(),
            query: None,
            context: None,
        }
    }

    /// Create a query error with SQL and context
    pub fn query_with_sql(message: impl Into<String>, query: impl Into<String>, context: Option<String>) -> Self {
        Self::Query {
            message: message.into(),
            query: Some(query.into()),
            context,
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field: None,
            value: None,
        }
    }

    /// Create a validation error with field context
    pub fn validation_field(message: impl Into<String>, field: impl Into<String>, value: Option<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field: Some(field.into()),
            value,
        }
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            table: None,
            key: None,
        }
    }

    /// Create a not found error with table and key context
    pub fn not_found_record(message: impl Into<String>, table: impl Into<String>, key: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            table: Some(table.into()),
            key: Some(key.into()),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
            field: None,
            source: None,
        }
    }

    /// Create a serialization error with field context
    pub fn serialization_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
            field: Some(field.into()),
            source: None,
        }
    }

    /// Create a migration error
    pub fn migration(message: impl Into<String>, table: Option<String>, operation: Option<String>) -> Self {
        Self::Migration {
            message: message.into(),
            table,
            operation,
            source: None,
        }
    }

    /// Create a type conversion error
    pub fn type_conversion(message: impl Into<String>, from_type: impl Into<String>, to_type: impl Into<String>) -> Self {
        Self::TypeConversion {
            message: message.into(),
            from_type: from_type.into(),
            to_type: to_type.into(),
        }
    }

    /// Create a constraint violation error
    pub fn constraint(message: impl Into<String>, constraint_type: Option<String>, table: Option<String>, column: Option<String>) -> Self {
        Self::Constraint {
            message: message.into(),
            constraint_type,
            table,
            column,
        }
    }

    /// Create a pagination error
    pub fn pagination(message: impl Into<String>, page: Option<u32>, per_page: Option<u32>) -> Self {
        Self::Pagination {
            message: message.into(),
            page,
            per_page,
        }
    }

    /// Create an operation error
    pub fn operation(message: impl Into<String>, operation: impl Into<String>, table: Option<String>) -> Self {
        Self::Operation {
            message: message.into(),
            operation: operation.into(),
            table,
            source: None,
        }
    }

    /// Create a DateTime error
    pub fn datetime(message: impl Into<String>, input: Option<String>, format: Option<String>) -> Self {
        Self::DateTime {
            message: message.into(),
            input,
            format,
        }
    }

    /// Create an internal error with location
    pub fn internal(message: impl Into<String>, location: Option<String>) -> Self {
        Self::Internal {
            message: message.into(),
            location,
        }
    }
}

// === From Implementations for External Error Types ===

impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Self {
        // Extract PostgreSQL error code if available
        let code = err.code().map(|c| c.code().to_string());

        Self::PostgreSql {
            message: err.to_string(),
            code,
            source: Some(Box::new(err)),
        }
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        Self::Pool {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            message: err.to_string(),
            field: None,
            source: Some(Box::new(err)),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
            operation: None,
            source: Some(Box::new(err)),
        }
    }
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Self {
        Self::DateTime {
            message: format!("DateTime parsing failed: {}", err),
            input: None,
            format: None,
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal {
            message: err.to_string(),
            location: None,
        }
    }
}

// For backward compatibility during transition
impl Error {
    /// Legacy method for serde deserialization errors
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            location: Some("serde".to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod database;
pub mod error;
pub mod filters;
pub mod macros;
pub mod migrations;
pub mod operations;
pub mod pagination;
pub mod query;
pub mod traits;
pub mod types;
pub mod utils;

#[cfg(test)]
mod test;

// Re-export PostgreSQL dependencies for macro use
pub use tokio_postgres;
pub use postgres_types;
// Re-export indexmap for ordered field preservation
pub use indexmap;

// Create orso module alias for macro compatibility
pub mod orso {
    pub use crate::*;
}

pub use chrono;
pub use cydec::{FloatingCodec, IntegerCodec};
pub use database::*;
pub use error::{Error, Result};
pub use filters::{Filter, FilterOperations, FilterOperator, FilterValue, SearchFilter, Sort};
pub use migrations::{MigrationEntry, MigrationResult, MigrationTrait, Migrations};
pub use orso_postgres_macros::{orso_column, orso_table, Orso};
pub use pagination::{CursorPaginatedResult, CursorPagination, PaginatedResult, Pagination};
pub use query::{QueryBuilder, QueryResult};
pub use serde::{Deserialize, Serialize};
pub use traits::{FieldType, Orso};
pub use types::*;
pub use types::OrsoDateTime;
pub use utils::Utils;
pub use uuid::Uuid;

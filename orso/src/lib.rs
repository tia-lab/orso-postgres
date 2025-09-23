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

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod test_sqlite;

// Re-export libsql and rusqlite for macro use
#[cfg(feature = "libsql")]
pub use libsql;
#[cfg(feature = "sqlite")]
pub use rusqlite;

pub use chrono;
pub use cydec::{FloatingCodec, IntegerCodec};
pub use database::*;
pub use error::{Error, Result};
pub use filters::{Filter, FilterOperations, FilterOperator, FilterValue, SearchFilter, Sort};
pub use migrations::{MigrationEntry, MigrationResult, MigrationTrait, Migrations};
pub use orso_macros::{orso_column, orso_table, Orso};
pub use pagination::{CursorPaginatedResult, CursorPagination, PaginatedResult, Pagination};
pub use query::{QueryBuilder, QueryResult};
pub use serde::{Deserialize, Serialize};
pub use traits::{FieldType, Orso};
pub use types::*;
pub use utils::Utils;
pub use uuid::Uuid;

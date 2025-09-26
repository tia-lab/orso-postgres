# orso-postgres

A PostgreSQL ORM for Rust with compression support, vector search capabilities, and automatic migrations.

## Overview

orso-postgres is a PostgreSQL adapter that provides derive-based schema definition, data compression for integer arrays, vector search integration, and CRUD operations. It uses `tokio-postgres` for async database operations and includes connection pooling for concurrent applications.

## Features

- **Derive macros**: Generate database schema from Rust structs using `#[derive(Orso)]`
- **PostgreSQL integration**: Built on `tokio-postgres` with connection pooling via `deadpool-postgres`
- **Data compression**: Compress integer arrays to reduce storage size using cydec codec
- **Vector search**: Support for PostgreSQL vector operations with pgvector extension
- **Automatic migrations**: Schema changes with data preservation
- **DateTime support**: PostgreSQL timestamp handling with `OrsoDateTime` wrapper
- **Query building**: Filtering, sorting, pagination, and custom queries
- **Batch operations**: Bulk insert/update/delete operations
- **Multi-table operations**: Use one struct definition across multiple tables

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
orso-postgres = "0.0.2"
```

## Quick Start

### 1. Define Your Model

```rust
use orso_postgres::{Orso, OrsoDateTime};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug)]
#[orso_table("users")]
struct User {
    #[orso_column(primary_key)]
    id: Option<String>,

    #[orso_column(unique)]
    email: String,

    name: String,
    age: i32,

    // DateTime wrapper for PostgreSQL timestamp handling
    birth_date: OrsoDateTime,

    // chrono::DateTime also supported
    last_login: Option<chrono::DateTime<chrono::Utc>>,

    #[orso_column(created_at)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,

    #[orso_column(updated_at)]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

### 2. Database Connection

```rust
use orso_postgres::{Database, DatabaseConfig};

// PostgreSQL connection
let config = DatabaseConfig::postgres("postgresql://user:password@localhost:5432/database")
    .with_pool_size(16);

let db = Database::init(config).await?;
```

### 3. Run Migrations

```rust
use orso_postgres::{Migrations, migration};

// Create tables with automatic migrations
Migrations::init(&db, &[migration!(User)]).await?;
```

### 4. CRUD Operations

```rust
// Create
let user = User {
    id: None,
    email: "john@example.com".to_string(),
    name: "John Doe".to_string(),
    age: 30,
    birth_date: OrsoDateTime::now(),
    last_login: Some(chrono::Utc::now()),
    created_at: None, // Auto-managed
    updated_at: None, // Auto-managed
};
user.insert(&db).await?;

// Read
let user = User::find_by_id("user-id", &db).await?;
let all_users = User::find_all(&db).await?;

// Update
if let Some(mut user) = User::find_by_id("user-id", &db).await? {
    user.age = 31;
    user.update(&db).await?;
}

// Delete
User::delete_by_id("user-id", &db).await?;
```

## PostgreSQL-Specific Features

### Connection Configuration

```rust
use orso_postgres::{Database, DatabaseConfig};

// Basic connection
let config = DatabaseConfig::new("postgresql://localhost/mydb");

// With connection pooling
let config = DatabaseConfig::postgres("postgresql://user:password@localhost:5432/db")
    .with_pool_size(32); // Configure connection pool size

let db = Database::init(config).await?;
```

### Supported PostgreSQL Types

| Rust Type                 | PostgreSQL Type         |
|---------------------------|-------------------------|
| `String`                  | TEXT                    |
| `i32`, `i16`, `i8`        | INTEGER                 |
| `i64`, `u64`              | BIGINT                  |
| `u32`, `u16`, `u8`        | INTEGER                 |
| `f64`, `f32`              | DOUBLE PRECISION        |
| `bool`                    | BOOLEAN                 |
| `Vec<u8>`                 | BYTEA                   |
| `OrsoDateTime`            | TIMESTAMP               |
| `chrono::DateTime<Utc>`   | TIMESTAMP               |
| `Vec<f32>`                | vector(N)               |
| `Vec<i32>` (compressed)   | BYTEA                   |
| `Vec<i64>` (compressed)   | BYTEA                   |
| `Vec<i32>` (normal)       | INTEGER[]               |
| `Vec<i64>` (normal)       | BIGINT[]                |
| `Vec<f64>` (normal)       | DOUBLE PRECISION[]      |
| `Option<T>`               | T (nullable)            |

### Vector Search

Vector search support for embeddings and machine learning applications using PostgreSQL's pgvector extension:

```rust
use orso_postgres::{QueryBuilder, Orso};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug)]
#[orso_table("documents")]
struct Document {
    #[orso_column(primary_key)]
    id: Option<String>,

    title: String,
    content: String,

    // Vector field for content embeddings (1536 dimensions for OpenAI ada-002)
    #[orso_column(vector(1536))]
    content_embedding: Vec<f32>,

    // Smaller vector for title embeddings
    #[orso_column(vector(768))]
    title_embedding: Vec<f32>,
}

// Vector similarity search
let query_vector = vec![0.1f32; 1536];
let similar_docs = QueryBuilder::new("documents")
    .vector_search("content_embedding", &query_vector, 10)
    .execute::<Document>(&db)
    .await?;

// Vector similarity with threshold
let filtered_docs = QueryBuilder::new("documents")
    .vector_similar("content_embedding", &query_vector, Some(0.8))
    .execute::<Document>(&db)
    .await?;

// Custom distance operators (<->, <#>, <=>)
let distance_query = QueryBuilder::new("documents")
    .vector_distance("content_embedding", &query_vector, "<->", Some(0.5))
    .execute::<Document>(&db)
    .await?;

// Hybrid text + vector search
let hybrid_results = QueryBuilder::new("documents")
    .search("content", "machine learning")
    .vector_similar("content_embedding", &query_vector, Some(0.8))
    .limit(5)
    .execute::<Document>(&db)
    .await?;
```

**Note**: Requires pgvector extension: `CREATE EXTENSION vector;`

### DateTime Handling

PostgreSQL timestamp support using the `OrsoDateTime` wrapper:

```rust
use orso_postgres::OrsoDateTime;

#[derive(Orso, Serialize, Deserialize, Clone, Debug)]
struct Event {
    // OrsoDateTime wrapper for consistent PostgreSQL formatting
    event_time: OrsoDateTime,

    // Direct chrono::DateTime usage also supported
    created_at: chrono::DateTime<chrono::Utc>,

    // Auto-managed timestamps
    #[orso_column(created_at)]
    auto_created: Option<chrono::DateTime<chrono::Utc>>,

    #[orso_column(updated_at)]
    auto_updated: Option<chrono::DateTime<chrono::Utc>>,
}

// Usage
let event = Event {
    event_time: OrsoDateTime::now(),
    created_at: chrono::Utc::now(),
    auto_created: None, // Automatically set by database
    auto_updated: None, // Automatically set by database
};
```

### PostgreSQL Arrays

Native PostgreSQL array support for non-compressed fields:

```rust
#[derive(Orso, Serialize, Deserialize, Clone, Debug)]
struct Analytics {
    // Stored as PostgreSQL INTEGER[] array
    scores: Vec<i32>,

    // Stored as PostgreSQL BIGINT[] array
    timestamps: Vec<i64>,

    // Stored as PostgreSQL DOUBLE PRECISION[] array
    values: Vec<f64>,
}
```

## Data Compression

Compress large integer arrays for significant space savings:

```rust
#[derive(Orso, Serialize, Deserialize, Clone, Debug)]
struct FinancialData {
    #[orso_column(primary_key)]
    id: Option<String>,

    symbol: String,

    // Compress large arrays with 5-10x space reduction
    #[orso_column(compress)]
    price_history: Vec<i64>,

    #[orso_column(compress)]
    volume_data: Vec<u64>,

    #[orso_column(compress)]
    trade_sizes: Vec<i32>,
}

// Usage - compression/decompression is automatic
let data = FinancialData {
    id: None,
    symbol: "BTCUSDT".to_string(),
    price_history: (0..10_000).map(|i| 45000 + i).collect(), // 10k prices
    volume_data: (0..10_000).map(|i| 1000000 + i as u64).collect(),
    trade_sizes: (0..10_000).map(|i| 100 + i).collect(),
};

// Automatically compressed when stored
data.insert(&db).await?;

// Automatically decompressed when retrieved
let retrieved = FinancialData::find_by_id("some-id", &db).await?;
// All arrays are fully decompressed and accessible
```

**Compression Benefits:**
- **Space Efficiency**: 5-10x storage reduction for typical integer sequences
- **Performance**: Sub-millisecond compression/decompression
- **Transparency**: Automatic with zero code changes required
- **Type Support**: Works with `Vec<i64>`, `Vec<u64>`, `Vec<i32>`, `Vec<u32>`

## Migrations

### Automatic Migration System

```rust
use orso_postgres::{Migrations, migration, MigrationConfig};

// Default migrations
Migrations::init(&db, &[
    migration!(User),
    migration!(Product),
]).await?;

// Custom migration configuration
let config = MigrationConfig {
    max_backups_per_table: Some(5),
    backup_retention_days: Some(30),
    backup_suffix: Some("backup".to_string()),
};

Migrations::init_with_config(&db, &[
    migration!(User),
    migration!(Product, "products_v2"), // Custom table name
], &config).await?;
```

### Migration Process

When schema changes are detected:

1. **Analysis**: Compare current vs expected schema
2. **Backup**: Create `{table}_migration_{timestamp}` backup table
3. **Migration**: Transfer data to new schema (preserving all data)
4. **Replacement**: Atomically replace original table
5. **Cleanup**: Remove old backup tables based on retention policy

## Querying and Filtering

### Basic Queries

```rust
use orso_postgres::{filter, filter_op, sort, pagination};

// Find all
let users = User::find_all(&db).await?;

// Find by ID
let user = User::find_by_id("user-id", &db).await?;

// Simple filtering
let adults = User::find_where(
    filter_op!(filter!("age", orso_postgres::Operator::Ge, 18)),
    &db
).await?;

// Complex filtering
let filter = filter_op!(and,
    filter!("age", orso_postgres::Operator::Ge, 18),
    filter!("email", orso_postgres::Operator::Like, "%@company.com")
);
let company_adults = User::find_where(filter, &db).await?;

// Sorting and pagination
let pagination = pagination!(1, 20); // Page 1, 20 items
let sorted_users = User::find_where_paginated_sorted(
    filter_op!(filter!("active", orso_postgres::Operator::Eq, true)),
    vec![sort!("name", asc)],
    &pagination,
    &db
).await?;
```

### Advanced Queries

```rust
use orso_postgres::{QueryBuilder, Aggregate, JoinType};

// Query builder
let results = QueryBuilder::new("users")
    .select(vec!["name", "email", "age"])
    ._where(filter_op!(filter!("age", orso_postgres::Operator::Ge, 18)))
    .order_by(sort!("name", asc))
    .limit(10)
    .execute::<User>(&db)
    .await?;

// Aggregation
let count = QueryBuilder::new("users")
    .aggregate(Aggregate::Count, "*", None)
    .execute_aggregate(&db)
    .await?;

// Joins
let results = QueryBuilder::new("users")
    .join(JoinType::Inner, "profiles", "users.id = profiles.user_id")
    .select(vec!["users.name", "profiles.bio"])
    .execute::<UserProfile>(&db)
    .await?;
```

## Batch Operations

Optimize performance with bulk operations:

```rust
// Batch insert
let users = vec![user1, user2, user3];
User::batch_insert(&users, &db).await?;

// Batch update
User::batch_update(&users, &db).await?;

// Batch delete
let ids = vec!["id1", "id2", "id3"];
User::batch_delete(&ids, &db).await?;

// Batch operations with custom table
User::batch_insert_with_table(&users, &db, "users_archive").await?;
```

## Multi-Table Operations

Use one struct with multiple tables:

```rust
// Runtime table selection
user.insert_with_table(&db, "users_archive").await?;
let archived_user = User::find_by_id_with_table("user-id", &db, "users_archive").await?;
user.update_with_table(&db, "users_temp").await?;
user.delete_with_table(&db, "users_old").await?;

// Batch operations with custom tables
User::batch_insert_with_table(&users, &db, "users_2024").await?;
let count = User::count_with_table(&db, "users_archive").await?;

// Create multiple tables from one struct
Migrations::init(&db, &[
    migration!(User, "users_current"),
    migration!(User, "users_archive"),
    migration!(User, "users_backup"),
]).await?;
```

## Utility Operations

Efficient operations for common patterns:

```rust
// Existence checks
let has_users = User::exists(&db).await?;
let has_adults = User::exists_filter(
    filter_op!(filter!("age", orso_postgres::Operator::Ge, 18)),
    &db
).await?;

// Find by any field
let johns = User::find_by_field("name",
    orso_postgres::Value::Text("John".to_string()), &db).await?;

// Find latest/first records
let latest_user = User::find_latest(&db).await?;
let oldest_user = User::find_first(&db).await?;

// Batch ID operations
let ids = vec!["id1", "id2", "id3"];
let users = User::find_by_ids(&ids, &db).await?;

// Field-based batch queries
let ages = vec![orso_postgres::Value::Integer(25), orso_postgres::Value::Integer(30)];
let users_25_or_30 = User::find_by_field_in("age", &ages, &db).await?;
```

## Error Handling

```rust
use orso_postgres::{Error, Result};

// Comprehensive error types
pub enum Error {
    Connection(String),    // Connection pool errors
    Sql(String),          // PostgreSQL errors
    Serialization(String), // JSON/serde errors
    Validation(String),    // Data validation errors
    NotFound(String),     // Record not found
    Query(String),        // Query building errors
    Config(String),       // Configuration errors
    // ... more
}

pub type Result<T> = std::result::Result<T, Error>;

// Usage
match User::find_by_id("user-id", &db).await {
    Ok(Some(user)) => println!("Found user: {}", user.name),
    Ok(None) => println!("User not found"),
    Err(Error::Sql(msg)) => eprintln!("Database error: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Column Attributes

Available column attributes:

```rust
#[derive(Orso, Serialize, Deserialize, Clone, Debug)]
#[orso_table("products")]
struct Product {
    #[orso_column(primary_key)]
    id: Option<String>,

    #[orso_column(unique)]
    sku: String,

    #[orso_column(ref = "categories")]
    category_id: String,

    name: String,
    price: f64,

    #[orso_column(compress)]
    sales_history: Vec<i64>,

    #[orso_column(vector(384))]
    product_embedding: Vec<f32>,

    #[orso_column(created_at)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,

    #[orso_column(updated_at)]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

## Convenience Macros

Simplify common operations:

```rust
use orso_postgres::{filter, filter_op, sort, pagination, query, search};

// Filters
let eq_filter = filter!("age", orso_postgres::Operator::Eq, 25);
let range_filter = filter!("age", between, 18, 65);
let in_filter = filter!("status", in, vec!["active", "pending"]);
let null_filter = filter!("email", is_null);

// Filter combinations
let complex_filter = filter_op!(and,
    filter!("age", orso_postgres::Operator::Ge, 18),
    filter!("status", orso_postgres::Operator::Eq, "active")
);

// Sorting and pagination
let sort_by_name = sort!("name", asc);
let page_config = pagination!(1, 20);

// Query building
let query_builder = query!("users");

// Search
let search_filter = search!("john", "name", "email");
```

## Performance Considerations

### Connection Pooling

```rust
// Configure appropriate pool size for your workload
let config = DatabaseConfig::postgres("postgresql://...")
    .with_pool_size(32); // Adjust based on concurrent load

let db = Database::init(config).await?;
```

### Batch Operations

```rust
// Prefer batch operations for multiple records
let users: Vec<User> = // ... large dataset

// Efficient: Single transaction for all inserts
User::batch_insert(&users, &db).await?;

// Inefficient: Individual transactions
for user in &users {
    user.insert(&db).await?; // Avoid this pattern
}
```

### Compression Guidelines

```rust
// Use compression for large arrays (>100 elements typically)
#[orso_column(compress)]
large_dataset: Vec<i64>, // Good: 1000+ elements

// Skip compression for small arrays
small_flags: Vec<i32>, // Good: <100 elements, no compression needed
```

## Dependencies

Key dependencies and their purposes:

- `tokio-postgres` - Async PostgreSQL driver
- `deadpool-postgres` - Connection pooling
- `postgres-types` - PostgreSQL type system
- `serde` + `serde_json` - Serialization/deserialization
- `chrono` - DateTime handling
- `cydec` - Integer array compression
- `uuid` - UUID generation
- `tokio` - Async runtime
- `thiserror` + `anyhow` - Error handling

## Limitations

- PostgreSQL-specific (no SQLite/MySQL support)
- Requires PostgreSQL 12+ for full feature support
- Advanced PostgreSQL features may require manual SQL
- Schema changes require running migrations
- Complex multi-table joins may need custom queries

## Example: Complete Application

```rust
use orso_postgres::{Database, DatabaseConfig, Migrations, migration, Orso, OrsoDateTime};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug)]
#[orso_table("trading_data")]
struct TradingData {
    #[orso_column(primary_key)]
    id: Option<String>,

    #[orso_column(unique)]
    symbol: String,

    price: f64,
    volume: i64,

    // Compressed integer arrays for space efficiency
    #[orso_column(compress)]
    price_history: Vec<i64>,

    #[orso_column(compress)]
    volume_history: Vec<u64>,

    // Vector field for price embeddings (machine learning features)
    #[orso_column(vector(512))]
    price_embedding: Vec<f32>,

    timestamp: OrsoDateTime,

    #[orso_column(created_at)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,

    #[orso_column(updated_at)]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Database connection
    let config = DatabaseConfig::postgres("postgresql://localhost/trading")
        .with_pool_size(16);
    let db = Database::init(config).await?;

    // Run migrations
    Migrations::init(&db, &[migration!(TradingData)]).await?;

    // Create trading data with embeddings
    let price_features = vec![0.1f32; 512]; // Placeholder for ML features
    let btc_data = TradingData {
        id: None,
        symbol: "BTCUSDT".to_string(),
        price: 45000.0,
        volume: 1000000,
        price_history: (0..1000).map(|i| 45000 + i).collect(),
        volume_history: (0..1000).map(|i| 1000000 + i as u64).collect(),
        price_embedding: price_features,
        timestamp: OrsoDateTime::now(),
        created_at: None,
        updated_at: None,
    };

    // Insert data (compression and vector storage happen automatically)
    btc_data.insert(&db).await?;

    // Query data
    let all_symbols = TradingData::find_all(&db).await?;
    println!("Found {} trading symbols", all_symbols.len());

    // Find specific symbol
    use orso_postgres::{filter, filter_op, QueryBuilder};
    let btc_records = TradingData::find_where(
        filter_op!(filter!("symbol", orso_postgres::Operator::Eq, "BTCUSDT")),
        &db
    ).await?;

    for record in btc_records {
        println!("BTC Price: {}, History entries: {}, Vector dims: {}",
            record.price, record.price_history.len(), record.price_embedding.len());
    }

    // Vector similarity search for similar price patterns
    let query_vector = vec![0.15f32; 512];
    let similar_patterns = QueryBuilder::new("trading_data")
        .vector_search("price_embedding", &query_vector, 5)
        .execute::<TradingData>(&db)
        .await?;

    println!("Found {} similar trading patterns", similar_patterns.len());

    Ok(())
}
```

This example demonstrates connection setup, migrations, data compression, vector storage, and similarity search using orso-postgres.
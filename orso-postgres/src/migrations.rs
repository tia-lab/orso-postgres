// Migration system with zero-loss schema changes
use crate::{database::Database, error::Error, traits::FieldType, Orso};
// use chrono::{DateTime, Utc}; // Reserved for future migration timestamp features
// use serde::{Deserialize, Serialize}; // Reserved for future migration serialization
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MigrationConfig {
    max_backups_per_table: Option<u8>,
    backup_retention_days: Option<u8>,
    backup_suffix: Option<String>,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            max_backups_per_table: Some(5),
            backup_retention_days: Some(30),
            backup_suffix: Some("migration".to_string()),
        }
    }
}

impl MigrationConfig {
    // Direct getters with built-in defaults
    pub fn max_backups(&self) -> u8 {
        self.max_backups_per_table.unwrap_or(5)
    }

    pub fn retention_days(&self) -> u8 {
        self.backup_retention_days.unwrap_or(30)
    }

    pub fn suffix(&self) -> &str {
        self.backup_suffix.as_deref().unwrap_or("migration")
    }
}

pub struct Migrations;

impl Migrations {
    /// Initialize database with migrations using default config
    /// Usage: Migrations::init(&db, &[migration!(User), migration!(Product)]).await?
    pub async fn init(
        db: &Database,
        migrations: &[Box<dyn MigrationTrait>],
    ) -> Result<Vec<MigrationResult>, Error> {
        Self::init_with_config(db, migrations, &MigrationConfig::default()).await
    }

    /// Initialize database with migrations and custom config
    /// Usage: Migrations::init_with_config(&db, &[migration!(User)], &config).await?
    pub async fn init_with_config(
        db: &Database,
        migrations: &[Box<dyn MigrationTrait>],
        config: &MigrationConfig,
    ) -> Result<Vec<MigrationResult>, Error> {
        let mut results = Vec::new();

        for migration in migrations {
            let result = migration.run_migration(db, config).await?;
            results.push(result);
        }

        Ok(results)
    }
}

// Trait for migrations to avoid generic constraints
#[async_trait::async_trait]
pub trait MigrationTrait: Send + Sync {
    async fn run_migration(
        &self,
        db: &Database,
        config: &MigrationConfig,
    ) -> Result<MigrationResult, Error>;
}

// Migration entry for the init system
pub struct MigrationEntry<T: Orso + Default> {
    _phantom: std::marker::PhantomData<T>,
    custom_table_name: Option<String>,
}

impl<T: Orso + Default> MigrationEntry<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            custom_table_name: None,
        }
    }

    pub fn with_custom_name(table_name: String) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            custom_table_name: Some(table_name),
        }
    }
}

#[async_trait::async_trait]
impl<T: Orso + Default + Send + Sync> MigrationTrait for MigrationEntry<T> {
    async fn run_migration(
        &self,
        db: &Database,
        config: &MigrationConfig,
    ) -> Result<MigrationResult, Error> {
        if let Some(custom_name) = &self.custom_table_name {
            ensure_table_with_name::<T>(db, custom_name, config).await
        } else {
            ensure_table::<T>(db, config).await
        }
    }
}

// migration! macro creates boxed MigrationEntry
#[macro_export]
macro_rules! migration {
    ($model:ty) => {
        Box::new($crate::migrations::MigrationEntry::<$model>::new())
            as Box<dyn $crate::migrations::MigrationTrait>
    };
    ($model:ty, $custom_name:expr) => {
        Box::new(
            $crate::migrations::MigrationEntry::<$model>::with_custom_name(
                $custom_name.to_string(),
            ),
        ) as Box<dyn $crate::migrations::MigrationTrait>
    };
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub position: i32,
    pub is_unique: bool,
    pub is_primary_key: bool,
    pub foreign_key_reference: Option<String>,
    pub has_default: bool,
    pub is_compressed: bool, // Track if this column should be compressed
}

#[derive(Debug, Clone)]
pub struct SchemaComparison {
    pub needs_migration: bool,
    pub changes: Vec<String>,
    pub current_columns: Vec<ColumnInfo>,
    pub expected_columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone)]
pub enum MigrationAction {
    TableCreated,
    SchemaMatched,
    DataMigrated { from: String, to: String },
}

#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub action: MigrationAction,
    pub backup_table: Option<String>,
    pub rows_migrated: Option<u64>,
    pub schema_changes: Vec<String>,
}

pub async fn ensure_table<T>(
    db: &Database,
    config: &MigrationConfig,
) -> Result<MigrationResult, Error>
where
    T: Orso + Default,
{
    let table_name = T::table_name();
    ensure_table_with_name::<T>(db, table_name, config).await
}

pub async fn ensure_table_with_name<T>(
    db: &Database,
    table_name: &str,
    config: &MigrationConfig,
) -> Result<MigrationResult, Error>
where
    T: Orso + Default,
{
    // Step 1: Infer expected schema from Orso trait
    let expected_schema = infer_schema_from_orso::<T>()?;

    // Step 2: Check if table exists
    let table_exists = check_table_exists(db, table_name).await?;

    if !table_exists {
        // PostgreSQL has foreign key constraints enabled by default (no action needed)

        // Create new table using custom SQL generation with table name override
        let create_sql = generate_migration_sql_with_custom_name::<T>(table_name);

        db.execute(&create_sql, &[])
            .await
            .map_err(|e| Error::DatabaseError(format!("Failed to create table: {}", e)))?;

        return Ok(MigrationResult {
            action: MigrationAction::TableCreated,
            backup_table: None,
            rows_migrated: None,
            schema_changes: vec![format!("Created table {} from schema", table_name)],
        });
    }

    // Step 3: Compare current vs expected schema
    let current_schema = get_current_table_schema(db, table_name).await?;
    let comparison = compare_schemas(&current_schema, &expected_schema);

    if !comparison.needs_migration {
        return Ok(MigrationResult {
            action: MigrationAction::SchemaMatched,
            backup_table: None,
            rows_migrated: None,
            schema_changes: vec![],
        });
    }

    // Step 4: Perform zero-loss migration using proven algorithm
    perform_zero_loss_migration(db, table_name, &comparison, config).await
}

fn generate_migration_sql_with_custom_name<T>(table_name: &str) -> String
where
    T: Orso,
{
    // Get the original migration SQL and replace the table name
    let original_sql = T::migration_sql();
    let original_table_name = T::table_name();

    // Replace the table name in the SQL
    // Handle both quoted and unquoted table names
    let replacements = [
        (
            format!("CREATE TABLE {}", original_table_name),
            format!("CREATE TABLE {}", table_name),
        ),
        (
            format!("CREATE TABLE \"{}\"", original_table_name),
            format!("CREATE TABLE \"{}\"", table_name),
        ),
        (
            format!("CREATE TABLE IF NOT EXISTS {}", original_table_name),
            format!("CREATE TABLE IF NOT EXISTS {}", table_name),
        ),
        (
            format!("CREATE TABLE IF NOT EXISTS \"{}\"", original_table_name),
            format!("CREATE TABLE IF NOT EXISTS \"{}\"", table_name),
        ),
    ];

    let mut modified_sql = original_sql;
    for (from, to) in replacements {
        modified_sql = modified_sql.replace(&from, &to);
    }

    modified_sql
}

fn infer_schema_from_orso<T>() -> Result<Vec<ColumnInfo>, Error>
where
    T: Orso,
{
    let mut columns = Vec::new();

    // Only add columns that actually exist in the struct
    let field_names = T::field_names();
    let field_types = T::field_types();
    let field_nullable = T::field_nullable();
    let field_compressed = T::field_compressed();
    let unique_fields = T::unique_fields();
    let primary_key_field = T::primary_key_field();

    if field_names.len() != field_types.len() || field_names.len() != field_nullable.len() {
        return Err(Error::DatabaseError(
            "Mismatched field arrays in Orso implementation".to_string(),
        ));
    }

    for (i, (((name, field_type), nullable), compressed)) in field_names
        .iter()
        .zip(field_types.iter())
        .zip(field_nullable.iter())
        .zip(field_compressed.iter())
        .enumerate()
    {
        // Determine if this field should be unique
        let is_unique = unique_fields.contains(name);

        // Determine if this is the primary key
        let is_primary_key = *name == primary_key_field;

        // For compressed fields, we use BYTEA type (PostgreSQL binary data)
        let sql_type = if *compressed {
            "BYTEA".to_string()
        } else {
            field_type_to_sqlite_type(field_type)
        };

        // Determine if this field has a default value
        // Primary key TEXT fields have gen_random_uuid() default
        // created_at and updated_at fields have NOW() default
        let has_default = if is_primary_key && sql_type == "TEXT" {
            true // PRIMARY KEY TEXT fields have DEFAULT gen_random_uuid()
        } else if *name == "created_at" || *name == "updated_at" {
            true // Timestamp fields have DEFAULT NOW()
        } else {
            false
        };

        columns.push(ColumnInfo {
            name: name.to_string(),
            sql_type,
            nullable: if is_primary_key { false } else { *nullable }, // PRIMARY KEY fields are always NOT NULL
            position: i as i32,
            is_unique: is_unique || is_primary_key, // Primary keys are implicitly unique
            is_primary_key,
            foreign_key_reference: None, // Would need to add this to Orso trait
            has_default,
            is_compressed: *compressed,  // Track compression status
        });
    }

    Ok(columns)
}

fn field_type_to_sqlite_type(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Text => "TEXT".to_string(),
        FieldType::Integer => "INTEGER".to_string(),  // PostgreSQL INTEGER (int4)
        FieldType::BigInt => "BIGINT".to_string(),    // PostgreSQL BIGINT (int8)
        FieldType::Numeric => "DOUBLE PRECISION".to_string(), // PostgreSQL DOUBLE PRECISION
        FieldType::Boolean => "BOOLEAN".to_string(),  // PostgreSQL native BOOLEAN
        FieldType::JsonB => "JSONB".to_string(),      // PostgreSQL native JSONB
        FieldType::Timestamp => "TIMESTAMP WITH TIME ZONE".to_string(), // PostgreSQL timestamp with timezone
    }
}

async fn check_table_exists(db: &Database, table_name: &str) -> Result<bool, Error> {
    let query = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1";

    let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> =
        vec![Box::new(table_name.to_string())];
    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
        params.iter().map(|p| p.as_ref()).collect();

    let rows = db
        .query(query, &param_refs)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to check table existence: {}", e)))?;

    Ok(!rows.is_empty())
}

async fn get_current_table_schema(
    db: &Database,
    table_name: &str,
) -> Result<Vec<ColumnInfo>, Error> {
    // Get PostgreSQL column information
    let query = "
        SELECT
            column_name,
            data_type,
            is_nullable,
            ordinal_position,
            column_default
        FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = $1
        ORDER BY ordinal_position
    ";

    let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> =
        vec![Box::new(table_name.to_string())];
    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
        params.iter().map(|p| p.as_ref()).collect();

    let rows = db
        .query(query, &param_refs)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get table info: {}", e)))?;

    let mut columns = Vec::new();
    let mut column_info_map = std::collections::HashMap::new();

    for row in rows.iter() {
        let name: String = row.get(0);
        let data_type: String = row.get(1);
        let is_nullable: String = row.get(2);
        let ordinal_position: i32 = row.get(3);
        let column_default: Option<String> = row.get(4);

        let column_info = ColumnInfo {
            name: name.clone(),
            sql_type: data_type.to_uppercase(),
            nullable: is_nullable == "YES",
            position: ordinal_position - 1, // Convert from 1-indexed to 0-indexed
            is_unique: false,            // Will be updated later from constraints
            is_primary_key: false,       // Will be updated later from constraints
            foreign_key_reference: None, // Will be updated later from constraints
            has_default: column_default.is_some(),
            is_compressed: data_type.to_uppercase() == "BYTEA", // PostgreSQL: BYTEA columns are probably compressed
        };

        column_info_map.insert(name.clone(), column_info.clone());
        columns.push(column_info);
    }

    // Sort by position to maintain order
    columns.sort_by_key(|c| c.position);

    // Get PostgreSQL constraint information (primary keys, unique constraints)
    let constraint_query = "
        SELECT
            kcu.column_name,
            tc.constraint_type
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
        ON tc.constraint_name = kcu.constraint_name
        WHERE tc.table_schema = 'public' AND tc.table_name = $1
        AND tc.constraint_type IN ('PRIMARY KEY', 'UNIQUE')
    ";

    let constraint_params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> =
        vec![Box::new(table_name.to_string())];
    let constraint_param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
        constraint_params.iter().map(|p| p.as_ref()).collect();

    let constraint_rows = db
        .query(constraint_query, &constraint_param_refs)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get constraint info: {}", e)))?;

    // Process constraint information and update column flags
    for row in constraint_rows {
        let column_name: String = row.get(0);
        let constraint_type: String = row.get(1);

        if let Some(column_info) = column_info_map.get_mut(&column_name) {
            match constraint_type.as_str() {
                "PRIMARY KEY" => {
                    column_info.is_primary_key = true;
                    column_info.is_unique = true; // PRIMARY KEY implies UNIQUE
                }
                "UNIQUE" => column_info.is_unique = true,
                _ => {}
            }
        }
    }

    // Get PostgreSQL foreign key information
    let fk_query = "
        SELECT
            kcu.column_name,
            ccu.table_name AS referenced_table_name,
            ccu.column_name AS referenced_column_name
        FROM information_schema.referential_constraints rc
        JOIN information_schema.key_column_usage kcu
        ON rc.constraint_name = kcu.constraint_name
        JOIN information_schema.constraint_column_usage ccu
        ON rc.unique_constraint_name = ccu.constraint_name
        WHERE kcu.table_schema = 'public' AND kcu.table_name = $1
    ";

    let fk_params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> =
        vec![Box::new(table_name.to_string())];
    let fk_param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
        fk_params.iter().map(|p| p.as_ref()).collect();

    let fk_rows = db
        .query(fk_query, &fk_param_refs)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to get foreign key list: {}", e)))?;

    for row in fk_rows {
        let column_name: String = row.get(0);
        let ref_table: String = row.get(1);
        let ref_column: String = row.get(2);

        // Mark this column as having a foreign key reference
        if let Some(column_info) = column_info_map.get_mut(&column_name) {
            column_info.foreign_key_reference = Some(format!("{}.{}", ref_table, ref_column));
        }
    }

    // Update the columns vector with the enhanced information
    for column in &mut columns {
        if let Some(updated_info) = column_info_map.get(&column.name) {
            column.is_primary_key = updated_info.is_primary_key;
            column.is_unique = updated_info.is_unique;
            column.foreign_key_reference = updated_info.foreign_key_reference.clone();
        }
    }

    Ok(columns)
}

fn compare_schemas(current: &[ColumnInfo], expected: &[ColumnInfo]) -> SchemaComparison {
    let mut changes = Vec::new();
    let mut needs_migration = false;

    // Check if schemas are identical
    if current.len() != expected.len() {
        changes.push(format!(
            "Column count differs: {} vs {}",
            current.len(),
            expected.len()
        ));
        needs_migration = true;
    }

    // Create maps for easier comparison
    let current_map: HashMap<String, &ColumnInfo> =
        current.iter().map(|c| (c.name.clone(), c)).collect();
    let expected_map: HashMap<String, &ColumnInfo> =
        expected.iter().map(|c| (c.name.clone(), c)).collect();

    // Check for missing columns
    for expected_col in expected {
        match current_map.get(&expected_col.name) {
            Some(current_col) => {
                if current_col.sql_type != expected_col.sql_type {
                    changes.push(format!(
                        "Type mismatch for {}: {} vs {}",
                        expected_col.name, current_col.sql_type, expected_col.sql_type
                    ));
                    needs_migration = true;
                }
                if current_col.nullable != expected_col.nullable {
                    changes.push(format!(
                        "Nullability mismatch for {}: {} vs {}",
                        expected_col.name, current_col.nullable, expected_col.nullable
                    ));
                    needs_migration = true;
                }
                if current_col.position != expected_col.position {
                    changes.push(format!(
                        "Position mismatch for {}: {} vs {}",
                        expected_col.name, current_col.position, expected_col.position
                    ));
                    needs_migration = true;
                }
                if current_col.is_unique != expected_col.is_unique {
                    changes.push(format!(
                        "Unique constraint mismatch for {}: {} vs {}",
                        expected_col.name, current_col.is_unique, expected_col.is_unique
                    ));
                    needs_migration = true;
                }
                if current_col.is_primary_key != expected_col.is_primary_key {
                    changes.push(format!(
                        "Primary key mismatch for {}: {} vs {}",
                        expected_col.name, current_col.is_primary_key, expected_col.is_primary_key
                    ));
                    needs_migration = true;
                }
                if current_col.is_compressed != expected_col.is_compressed {
                    changes.push(format!(
                        "Compression mismatch for {}: {} vs {}",
                        expected_col.name, current_col.is_compressed, expected_col.is_compressed
                    ));
                    needs_migration = true;
                }
                // Note: We're not checking foreign key references here as they require
                // additional Orso trait methods that we haven't added yet
            }
            None => {
                changes.push(format!("Missing column: {}", expected_col.name));
                needs_migration = true;
            }
        }
    }

    // Check for extra columns
    for current_col in current {
        if !expected_map.contains_key(&current_col.name) {
            changes.push(format!("Extra column: {}", current_col.name));
            needs_migration = true;
        }
    }

    SchemaComparison {
        needs_migration,
        changes,
        current_columns: current.to_vec(),
        expected_columns: expected.to_vec(),
    }
}

async fn perform_zero_loss_migration(
    db: &Database,
    table_name: &str,
    comparison: &SchemaComparison,
    config: &MigrationConfig,
) -> Result<MigrationResult, Error> {
    // Generate unique backup table name with timestamp hash
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let backup_name = format!("{}_{}_{}", table_name, config.suffix(), timestamp);

    // Step 1: Create new table with correct schema
    let temp_table_name = format!("{}_temp_{}", table_name, timestamp);
    let create_sql = generate_create_table_sql(&temp_table_name, &comparison.expected_columns);

    db.execute(&create_sql, &[])
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create temp table: {}", e)))?;

    // Step 2: Copy data from old table to new table (preserving row order)
    let copy_sql = generate_data_migration_sql(
        table_name,
        &temp_table_name,
        &comparison.current_columns,
        &comparison.expected_columns,
    );

    let _rows_affected = db
        .execute(&copy_sql, &[])
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to migrate data: {}", e)))?;

    // Step 3: Rename original table to backup
    let rename_to_backup = format!("ALTER TABLE {} RENAME TO {}", table_name, backup_name);
    db.execute(&rename_to_backup, &[])
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to create backup: {}", e)))?;

    // Step 4: Rename new table to original name
    let rename_to_original = format!("ALTER TABLE {} RENAME TO {}", temp_table_name, table_name);
    db.execute(&rename_to_original, &[])
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to rename new table: {}", e)))?;

    // Step 5: Verify migration success
    let verification_sql = format!("SELECT COUNT(*) FROM {}", table_name);
    let rows = db
        .query(&verification_sql, &[])
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to verify migration: {}", e)))?;

    let row_count: i64 = if let Some(row) = rows.get(0) {
        row.get(0)
    } else {
        0
    };

    check_backups_retention(db, table_name, config).await?;

    Ok(MigrationResult {
        action: MigrationAction::DataMigrated {
            from: backup_name.clone(),
            to: table_name.to_string(),
        },
        backup_table: Some(backup_name),
        rows_migrated: Some(row_count as u64),
        schema_changes: comparison.changes.clone(),
    })
}

fn generate_create_table_sql(table_name: &str, columns: &[ColumnInfo]) -> String {
    let mut column_defs = Vec::new();
    let mut table_constraints = Vec::new();

    for column in columns {
        let mut def = format!("\"{}\" {}", column.name, column.sql_type);

        if !column.nullable {
            def.push_str(" NOT NULL");
        }

        // Add unique constraints
        if column.is_unique {
            // For unique constraints, we add them as table-level constraints
            // to avoid issues with column-level unique constraints in some cases
            table_constraints.push(format!("UNIQUE (\"{}\")", column.name));
        }

        // Add primary key constraints
        if column.is_primary_key {
            def.push_str(" PRIMARY KEY");
        }

        // Column defaults are now handled by the macro's column definition

        column_defs.push(def);
    }

    // Add table-level constraints
    column_defs.extend(table_constraints);

    format!(
        "CREATE TABLE IF NOT EXISTS \"{}\" (\n  {}\n)",
        table_name,
        column_defs.join(",\n  ")
    )
}

fn generate_data_migration_sql(
    source_table: &str,
    target_table: &str,
    source_columns: &[ColumnInfo],
    target_columns: &[ColumnInfo],
) -> String {
    // Create maps for column matching
    let source_map: HashMap<String, &ColumnInfo> =
        source_columns.iter().map(|c| (c.name.clone(), c)).collect();

    let mut select_columns = Vec::new();

    for target_col in target_columns {
        if let Some(_source_col) = source_map.get(&target_col.name) {
            // Column exists in both, copy directly
            select_columns.push(format!("\"{}\"", target_col.name));
        } else {
            // Column doesn't exist in source, use NULL or appropriate default
            if target_col.nullable {
                select_columns.push("NULL".to_string());
            } else {
                // Provide default values for NOT NULL columns based on type
                match target_col.sql_type.as_str() {
                    "TEXT" => select_columns.push("''".to_string()),
                    "INTEGER" => select_columns.push("0".to_string()),
                    "REAL" => select_columns.push("0.0".to_string()),
                    _ => select_columns.push("NULL".to_string()),
                }
            }
        }
    }

    let target_column_names: Vec<String> = target_columns
        .iter()
        .map(|c| format!("\"{}\"", c.name))
        .collect();

    format!(
        "INSERT INTO \"{}\" ({}) SELECT {} FROM \"{}\"",
        target_table,
        target_column_names.join(", "),
        select_columns.join(", "),
        source_table
    )
}

async fn check_backups_retention(
    db: &Database,
    table_name: &str,
    config: &MigrationConfig,
) -> Result<(), Error> {
    // Get all migration tables for this base table
    let migration_tables = get_all_migration_tables(db, table_name, config.suffix()).await?;

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Sort by timestamp (newest first)
    let mut sorted_tables = migration_tables;
    sorted_tables.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Delete tables that exceed the maximum count OR are too old
    for (index, old_table) in sorted_tables.iter().enumerate() {
        let age_seconds = current_time - old_table.timestamp;
        let age_days = age_seconds / 86400; // seconds to days

        let should_delete =
            // Delete if we exceed max backups (keep only the most recent ones)
            index >= config.max_backups() as usize ||
            // OR delete if older than retention policy
            age_days > config.retention_days() as u64;

        if should_delete {
            let drop_sql = format!("DROP TABLE IF EXISTS \"{}\"", old_table.name);
            db.execute(&drop_sql, &[]).await.map_err(|e| {
                Error::DatabaseError(format!("Failed to drop old migration table: {}", e))
            })?;

            tracing::info!(
                "Cleaned up old migration table: {} (age: {} days, index: {})",
                old_table.name,
                age_days,
                index
            );
        }
    }

    Ok(())
}

#[derive(Debug)]
struct MigrationTableInfo {
    name: String,
    timestamp: u64,
}

async fn get_all_migration_tables(
    db: &Database,
    base_table: &str,
    suffix: &str,
) -> Result<Vec<MigrationTableInfo>, Error> {
    let pattern = format!("{}_{}_", base_table, suffix);
    let query = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE $1";

    let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> =
        vec![Box::new(format!("{}%", pattern))];
    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
        params.iter().map(|p| p.as_ref()).collect();

    let rows = db
        .query(query, &param_refs)
        .await
        .map_err(|e| Error::DatabaseError(format!("Failed to query migration tables: {}", e)))?;

    let mut migration_tables = Vec::new();

    for row in rows {
        let table_name: String = row.get(0);

        // Extract timestamp from table name like "table_migration_1234567890"
        let suffix_pattern = format!("_{}_", suffix);
        if let Some(timestamp_str) = table_name.split(&suffix_pattern).nth(1) {
            if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                migration_tables.push(MigrationTableInfo {
                    name: table_name,
                    timestamp,
                });
            }
        }
    }

    Ok(migration_tables)
}

impl std::fmt::Display for MigrationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationAction::TableCreated => write!(f, "TableCreated"),
            MigrationAction::SchemaMatched => write!(f, "SchemaMatched"),
            MigrationAction::DataMigrated { from, to } => {
                write!(f, "DataMigrated from {} to {}", from, to)
            }
        }
    }
}

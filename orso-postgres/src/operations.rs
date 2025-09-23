use crate::{
    Aggregate, Database, Error, FilterOperator, PaginatedResult, Pagination, QueryBuilder, Result,
    SearchFilter, Sort, SortOrder,
};
use std::collections::HashMap;
use tracing::{debug, info, trace, warn};

/// CRUD operations for database models
pub struct CrudOperations;

impl CrudOperations {
    /// Insert a new record in the database
    pub async fn insert<T>(model: &T, db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::insert_with_table(model, db, T::table_name()).await
    }
    /// Insert a new record in the database
    pub async fn insert_with_table<T>(model: &T, db: &Database, table_name: &str) -> Result<()>
    where
        T: crate::Orso,
    {
        let map = model.to_map()?;
        let columns: Vec<String> = map.keys().cloned().collect();
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            placeholders.join(", ")
        );

        debug!(sql = %sql, "Executing SQL");

        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> = map
            .values()
            .map(|v| T::value_to_postgres_param(v))
            .collect();

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        db.execute(&sql, &param_refs).await?;

        debug!(table = table_name, "Successfully created record");
        Ok(())
    }

    /// Insert or update a record based on whether it has a primary key
    pub async fn insert_or_update<T>(model: &T, db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::insert_or_update_with_table(model, db, T::table_name()).await
    }

    pub async fn insert_or_update_with_table<T>(
        model: &T,
        db: &Database,
        table_name: &str,
    ) -> Result<()>
    where
        T: crate::Orso,
    {
        if let Some(id) = model.get_primary_key() {
            // Check if record exists
            match Self::find_by_id_with_table::<T>(&id, db, table_name).await? {
                Some(_) => {
                    // Record exists, update it
                    Self::update_with_table(model, db, table_name).await
                }
                None => {
                    // Record doesn't exist, insert it
                    warn!(table = table_name, id = %id, "Record with ID not found, creating new record");
                    Self::insert_with_table(model, db, table_name).await
                }
            }
        } else {
            // No primary key, insert new record
            trace!(
                table = table_name,
                "Creating new record (no primary key provided)"
            );
            Self::insert_with_table(model, db, table_name).await
        }
    }

    /// Insert or update a record based on unique constraints
    pub async fn upsert<T>(model: &T, db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::upsert_with_table(model, db, T::table_name()).await
    }

    pub async fn upsert_with_table<T>(model: &T, db: &Database, table_name: &str) -> Result<()>
    where
        T: crate::Orso,
    {
        let unique_columns: Vec<&str> = T::unique_fields();
        if unique_columns.is_empty() {
            return Err(Error::Validation(
                "No unique columns defined with orso_column(unique) for upsert".to_string(),
            ));
        }

        let map = model.to_map()?;

        // Build WHERE clause for unique columns
        let mut where_conditions = Vec::new();
        let mut where_params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> = Vec::new();

        for (param_index, column) in unique_columns.iter().enumerate() {
            if let Some(value) = map.get(*column) {
                where_conditions.push(format!("{column} = ${}", param_index + 1));
                where_params.push(T::value_to_postgres_param(value));
            }
        }

        if where_conditions.is_empty() {
            return Err(Error::Validation(
                "No valid unique column values found for upsert".to_string(),
            ));
        }

        let where_clause = where_conditions.join(" AND ");
        let sql = format!(
            "SELECT * FROM {} WHERE {} LIMIT 1",
            table_name, where_clause
        );

        info!(table = table_name, "Checking for existing record");
        debug!(sql = %sql, "Executing upsert query");

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            where_params.iter().map(|p| p.as_ref()).collect();

        let rows = db.query(&sql, &param_refs).await?;

        if !rows.is_empty() {
            // Record exists, update it
            let _row_map = T::row_to_map(&rows[0])?;
            info!(table = table_name, "Found existing record, updating");
            Self::update_with_table(model, db, table_name).await
        } else {
            // Record doesn't exist, insert it
            info!(
                table = table_name,
                "No existing record found, creating new one"
            );
            Self::insert_with_table(model, db, table_name).await
        }
    }

    /// Insert multiple records using Turso batch operations for optimal performance
    pub async fn batch_create<T>(models: &[T], db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::batch_insert_with_table(models, db, T::table_name()).await
    }

    pub async fn batch_insert_with_table<T>(
        models: &[T],
        db: &Database,
        table_name: &str,
    ) -> Result<()>
    where
        T: crate::Orso,
    {
        if models.is_empty() {
            return Ok(());
        }

        // Use proper parameterized queries instead of building SQL strings
        for model in models {
            let map = model.to_map()?;
            let columns: Vec<String> = map.keys().cloned().collect();
            let placeholders: Vec<String> =
                (1..=columns.len()).map(|i| format!("${}", i)).collect();

            let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> = map
                .values()
                .map(|v| T::value_to_postgres_param(v))
                .collect();

            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                columns.join(", "),
                placeholders.join(", ")
            );

            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
                params.iter().map(|p| p.as_ref()).collect();

            db.execute(&sql, &param_refs).await?;
        }
        Ok(())
    }

    /// Find a record by its primary key
    pub async fn find_by_id<T>(id: &str, db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_by_id_with_table(id, db, T::table_name()).await
    }

    pub async fn find_by_id_with_table<T>(
        id: &str,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = $1 LIMIT 1",
            table_name,
            T::primary_key_field() // Use dynamic primary key field name
        );

        debug!(table =table_name, id = %id, "Finding record by ID");
        debug!(sql = %sql, "Executing find query");

        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> =
            vec![Box::new(id.to_string())];

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let rows = db.query(&sql, &param_refs).await?;

        if let Some(row) = rows.get(0) {
            let map = T::row_to_map(&row)?;
            debug!(table =table_name, id = %id, "Found record");
            Ok(Some(T::from_map(map)?))
        } else {
            debug!(table =table_name, id = %id, "No record found");
            Ok(None)
        }
    }

    /// Find a single record by a specific condition
    pub async fn find_one<T>(filter: FilterOperator, db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_one_with_table(filter, db, T::table_name()).await
    }

    pub async fn find_one_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter).limit(1);

        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find all records
    pub async fn find_all<T>(db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_all_with_table(db, T::table_name()).await
    }

    pub async fn find_all_with_table<T>(db: &Database, table_name: &str) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name);
        builder.execute::<T>(db).await
    }

    /// Find records with a filter
    pub async fn find_where<T>(filter: FilterOperator, db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_where_with_table(filter, db, T::table_name()).await
    }

    pub async fn find_where_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute::<T>(db).await
    }

    pub async fn find_latest<T>(db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_latest_with_table(db, T::table_name()).await
    }

    pub async fn find_latest_with_table<T>(db: &Database, table_name: &str) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Desc);
        let builder = QueryBuilder::new(table_name).order_by(sort).limit(1);

        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find latest record matching filter
    pub async fn find_latest_filter<T>(filter: FilterOperator, db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_latest_filter_with_table(filter, db, T::table_name()).await
    }

    pub async fn find_latest_filter_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Desc);
        let builder = QueryBuilder::new(table_name)
            ._where(filter)
            .order_by(sort)
            .limit(1);
        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find first record matching filter (oldest)
    pub async fn find_first_filter<T>(filter: FilterOperator, db: &Database) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_first_filter_with_table(filter, db, T::table_name()).await
    }

    pub async fn find_first_filter_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Asc);
        let builder = QueryBuilder::new(table_name)
            ._where(filter)
            .order_by(sort)
            .limit(1);
        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Check if any record exists
    pub async fn exists<T>(db: &Database) -> Result<bool>
    where
        T: crate::Orso,
    {
        Self::exists_with_table::<T>(db, T::table_name()).await
    }

    pub async fn exists_with_table<T>(db: &Database, table_name: &str) -> Result<bool>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name).limit(1);
        let count = builder.execute_count(db).await?;
        Ok(count > 0)
    }

    /// Check if any record exists matching filter
    pub async fn exists_filter<T>(filter: FilterOperator, db: &Database) -> Result<bool>
    where
        T: crate::Orso,
    {
        Self::exists_filter_with_table::<T>(filter, db, T::table_name()).await
    }

    pub async fn exists_filter_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<bool>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter).limit(1);
        let count = builder.execute_count(db).await?;
        Ok(count > 0)
    }

    /// Find by any field value
    pub async fn find_by_field<T>(field: &str, value: crate::Value, db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_by_field_with_table(field, value, db, T::table_name()).await
    }

    pub async fn find_by_field_with_table<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        let filter =
            FilterOperator::Single(crate::Filter::new_simple(field, crate::Operator::Eq, value));
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute::<T>(db).await
    }

    /// Find latest record by field value
    pub async fn find_latest_by_field<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_latest_by_field_with_table(field, value, db, T::table_name()).await
    }

    pub async fn find_latest_by_field_with_table<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let filter =
            FilterOperator::Single(crate::Filter::new_simple(field, crate::Operator::Eq, value));
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Desc);
        let builder = QueryBuilder::new(table_name)
            ._where(filter)
            .order_by(sort)
            .limit(1);
        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find first record by field value (oldest)
    pub async fn find_first_by_field<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        Self::find_first_by_field_with_table(field, value, db, T::table_name()).await
    }

    pub async fn find_first_by_field_with_table<T>(
        field: &str,
        value: crate::Value,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<T>>
    where
        T: crate::Orso,
    {
        let filter =
            FilterOperator::Single(crate::Filter::new_simple(field, crate::Operator::Eq, value));
        let created_at_field = T::created_at_field().unwrap_or("created_at");
        let sort = Sort::new(created_at_field, SortOrder::Asc);
        let builder = QueryBuilder::new(table_name)
            ._where(filter)
            .order_by(sort)
            .limit(1);
        let results = builder.execute::<T>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Find multiple records by IDs (batch operation)
    pub async fn find_by_ids<T>(ids: &[&str], db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_by_ids_with_table(ids, db, T::table_name()).await
    }

    pub async fn find_by_ids_with_table<T>(
        ids: &[&str],
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let id_values: Vec<crate::Value> = ids
            .iter()
            .map(|id| crate::Value::Text(id.to_string()))
            .collect();
        let pk_field = T::primary_key_field();
        let filter = FilterOperator::Single(crate::Filter::in_values(pk_field, id_values));
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute::<T>(db).await
    }

    /// Find records by multiple values for same field (IN clause)
    pub async fn find_by_field_in<T>(
        field: &str,
        values: &[crate::Value],
        db: &Database,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        Self::find_by_field_in_with_table(field, values, db, T::table_name()).await
    }

    pub async fn find_by_field_in_with_table<T>(
        field: &str,
        values: &[crate::Value],
        db: &Database,
        table_name: &str,
    ) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        if values.is_empty() {
            return Ok(Vec::new());
        }

        let filter = FilterOperator::Single(crate::Filter::in_values(field, values.to_vec()));
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute::<T>(db).await
    }

    /// Find records with pagination
    pub async fn find_paginated<T>(
        pagination: &Pagination,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::find_paginated_with_table(pagination, db, T::table_name()).await
    }

    pub async fn find_paginated_with_table<T>(
        pagination: &Pagination,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name);
        builder.execute_paginated::<T>(db, pagination).await
    }

    /// Find records with filter and pagination
    pub async fn find_where_paginated<T>(
        filter: FilterOperator,
        pagination: &Pagination,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::find_where_paginated_with_table(filter, pagination, db, T::table_name()).await
    }

    pub async fn find_where_paginated_with_table<T>(
        filter: FilterOperator,
        pagination: &Pagination,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter);
        builder.execute_paginated::<T>(db, pagination).await
    }

    /// Search records with text search
    pub async fn search<T>(
        search_filter: &SearchFilter,
        pagination: Option<&Pagination>,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::search_with_table(search_filter, pagination, db, T::table_name()).await
    }

    pub async fn search_with_table<T>(
        search_filter: &SearchFilter,
        pagination: Option<&Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let filter = search_filter.to_filter_operator();
        let pagination = pagination.unwrap_or(&Pagination::default()).clone();

        Self::find_where_paginated_with_table::<T>(filter, &pagination, db, table_name).await
    }

    /// Count all records
    pub async fn count<T>(db: &Database) -> Result<u64>
    where
        T: crate::Orso,
    {
        Self::count_with_table::<T>(db, T::table_name()).await
    }

    pub async fn count_with_table<T>(db: &Database, table_name: &str) -> Result<u64>
    where
        T: crate::Orso,
    {
        let sql = format!("SELECT COUNT(*) FROM {}", table_name);
        let rows = db.query(&sql, &[]).await?;

        if let Some(row) = rows.get(0) {
            let count: i64 = row.get(0);
            Ok(count as u64)
        } else {
            Err(Error::Query("No count result".to_string()))
        }
    }

    /// Count records with a filter
    pub async fn count_where<T>(filter: FilterOperator, db: &Database) -> Result<u64>
    where
        T: crate::Orso,
    {
        Self::count_where_with_table::<T>(filter, db, T::table_name()).await
    }

    pub async fn count_where_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<u64>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter);

        let (sql, params) = builder.build_count()?;
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let rows = db.query(&sql, &param_refs).await?;

        if let Some(row) = rows.get(0) {
            let count: i64 = row.get(0);
            Ok(count as u64)
        } else {
            Err(Error::Query("No count result".to_string()))
        }
    }

    /// Update a record
    pub async fn update<T>(model: &T, db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::update_with_table(model, db, T::table_name()).await
    }

    pub async fn update_with_table<T>(model: &T, db: &Database, table_name: &str) -> Result<()>
    where
        T: crate::Orso,
    {
        let id = model.get_primary_key().ok_or_else(|| {
            Error::Validation("Cannot update record without primary key".to_string())
        })?;

        let map = model.to_map()?;
        let pk_field = T::primary_key_field();
        let updated_at_field = T::updated_at_field();

        let mut set_clauses = Vec::new();
        let mut param_index = 1;
        for k in map.keys() {
            if k != pk_field {
                // For updated_at fields, use database function instead of model value
                if updated_at_field.is_some() && k == updated_at_field.unwrap() {
                    set_clauses.push(format!("{k} = NOW()"));
                } else {
                    set_clauses.push(format!("{k} = ${}", param_index));
                    param_index += 1;
                }
            }
        }

        let sql = format!(
            "UPDATE {} SET {} WHERE {} = ${}",
            table_name,
            set_clauses.join(", "),
            pk_field,
            param_index
        );

        info!(table = table_name, id = %id, "Updating record");
        debug!(sql = %sql, "Executing update query");

        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> = map
            .iter()
            .filter(|(k, _)| {
                k != &pk_field && !(updated_at_field.is_some() && k == &updated_at_field.unwrap())
            })
            .map(|(_, v)| T::value_to_postgres_param(v))
            .collect();
        params.push(Box::new(id.clone()));

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        db.execute(&sql, &param_refs).await?;

        info!(table = table_name, id = %id, "Successfully updated record");
        Ok(())
    }

    /// Update multiple records using Turso batch operations
    pub async fn batch_update<T>(models: &[T], db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::batch_update_with_table(models, db, T::table_name()).await
    }

    pub async fn batch_update_with_table<T>(
        models: &[T],
        db: &Database,
        table_name: &str,
    ) -> Result<()>
    where
        T: crate::Orso,
    {
        if models.is_empty() {
            return Ok(());
        }

        for model in models {
            let id = model.get_primary_key().ok_or_else(|| {
                Error::Validation("Cannot batch update record without primary key".to_string())
            })?;

            let map = model.to_map()?;
            let pk_field = T::primary_key_field();
            let updated_at_field = T::updated_at_field();

            let mut set_clauses = Vec::new();
            let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> = Vec::new();
            let mut param_index = 1;

            for (k, v) in &map {
                if k != pk_field {
                    // For updated_at fields, use database function instead of model value
                    if updated_at_field.is_some() && k == updated_at_field.unwrap() {
                        set_clauses.push(format!("{} = NOW()", k));
                    } else {
                        set_clauses.push(format!("{} = ${}", k, param_index));
                        params.push(T::value_to_postgres_param(v));
                        param_index += 1;
                    }
                }
            }

            // Add the ID parameter for the WHERE clause
            params.push(Box::new(id.clone()));

            let sql = format!(
                "UPDATE {} SET {} WHERE {} = ${}",
                table_name,
                set_clauses.join(", "),
                pk_field,
                param_index
            );

            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
                params.iter().map(|p| p.as_ref()).collect();

            db.execute(&sql, &param_refs).await?;
        }
        Ok(())
    }

    /// Delete a record
    pub async fn delete<T>(model: &T, db: &Database) -> Result<bool>
    where
        T: crate::Orso,
    {
        Self::delete_with_table(model, db, T::table_name()).await
    }

    pub async fn delete_with_table<T>(model: &T, db: &Database, table_name: &str) -> Result<bool>
    where
        T: crate::Orso,
    {
        let id = model.get_primary_key().ok_or_else(|| {
            Error::Validation("Cannot delete record without primary key".to_string())
        })?;

        let sql = format!(
            "DELETE FROM {} WHERE {} = $1",
            table_name,
            T::primary_key_field()
        );

        info!(table = table_name, id = %id, "Deleting record");
        debug!(sql = %sql, "Executing delete query");

        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> = vec![Box::new(id)];

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        db.execute(&sql, &param_refs).await?;
        info!(table = table_name, "Successfully deleted record");
        Ok(true)
    }

    /// Delete multiple records using Turso batch operations
    pub async fn batch_delete<T>(ids: &[&str], db: &Database) -> Result<u64>
    where
        T: crate::Orso,
    {
        Self::batch_delete_with_table::<T>(ids, db, T::table_name()).await
    }

    pub async fn batch_delete_with_table<T>(
        ids: &[&str],
        db: &Database,
        table_name: &str,
    ) -> Result<u64>
    where
        T: crate::Orso,
    {
        if ids.is_empty() {
            return Ok(0);
        }

        let pk_field = T::primary_key_field();

        // Use IN clause for efficient bulk delete
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("${}", i)).collect();
        let sql = format!(
            "DELETE FROM {} WHERE {} IN ({})",
            table_name,
            pk_field,
            placeholders.join(", ")
        );

        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> = ids
            .iter()
            .map(|id| {
                Box::new(id.to_string()) as Box<dyn tokio_postgres::types::ToSql + Send + Sync>
            })
            .collect();

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let affected_rows = db.execute(&sql, &param_refs).await?;
        Ok(affected_rows)
    }

    /// Upsert multiple records using Turso batch operations with automatically detected unique columns
    pub async fn batch_upsert<T>(models: &[T], db: &Database) -> Result<()>
    where
        T: crate::Orso,
    {
        Self::batch_upsert_with_table(models, db, T::table_name()).await
    }

    pub async fn batch_upsert_with_table<T>(
        models: &[T],
        db: &Database,
        table_name: &str,
    ) -> Result<()>
    where
        T: crate::Orso,
    {
        if models.is_empty() {
            return Ok(());
        }

        let unique_columns: Vec<&str> = T::unique_fields();
        if unique_columns.is_empty() {
            return Err(Error::Validation(
                "No unique columns defined with orso_column(unique) for batch upsert".to_string(),
            ));
        }

        for model in models {
            let map = model.to_map()?;

            // Build conflict columns for ON CONFLICT clause
            let conflict_columns = unique_columns.join(", ");

            let columns: Vec<String> = map.keys().cloned().collect();
            let placeholders: Vec<String> =
                (1..=columns.len()).map(|i| format!("${}", i)).collect();

            let params: Vec<Box<dyn tokio_postgres::types::ToSql + Send + Sync>> = map
                .values()
                .map(|v| T::value_to_postgres_param(v))
                .collect();

            // Build UPDATE SET clause for conflict resolution
            let updated_at_field = T::updated_at_field();
            let update_sets: Vec<String> = columns
                .iter()
                .filter(|col| !unique_columns.contains(&col.as_str())) // Don't update unique columns
                .map(|col| {
                    // For updated_at fields, use database function instead of excluded value
                    if updated_at_field.is_some() && col == updated_at_field.unwrap() {
                        format!("{} = NOW()", col)
                    } else {
                        format!("{} = EXCLUDED.{}", col, col)
                    }
                })
                .collect();

            let sql = if update_sets.is_empty() {
                // If no columns to update, just ignore conflicts
                format!(
                    "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO NOTHING",
                    table_name,
                    columns.join(", "),
                    placeholders.join(", "),
                    conflict_columns
                )
            } else {
                // Use INSERT ... ON CONFLICT DO UPDATE for proper upsert
                format!(
                    "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
                    table_name,
                    columns.join(", "),
                    placeholders.join(", "),
                    conflict_columns,
                    update_sets.join(", ")
                )
            };

            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
                params.iter().map(|p| p.as_ref()).collect();

            db.execute(&sql, &param_refs).await?;
        }
        Ok(())
    }

    /// Delete records with a filter
    pub async fn delete_where<T>(filter: FilterOperator, db: &Database) -> Result<u64>
    where
        T: crate::Orso,
    {
        Self::delete_where_with_table::<T>(filter, db, T::table_name()).await
    }

    pub async fn delete_where_with_table<T>(
        filter: FilterOperator,
        db: &Database,
        table_name: &str,
    ) -> Result<u64>
    where
        T: crate::Orso,
    {
        let builder = QueryBuilder::new(table_name)._where(filter);

        let (sql, params) = builder.build()?;
        let delete_sql = sql.replace("SELECT *", "DELETE");

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let affected_rows = db.execute(&delete_sql, &param_refs).await?;
        Ok(affected_rows)
    }

    /// List records with optional sorting and pagination
    pub async fn list<T>(
        sort: Option<Vec<Sort>>,
        pagination: Option<&Pagination>,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::list_with_table(sort, pagination, db, T::table_name()).await
    }

    pub async fn list_with_table<T>(
        sort: Option<Vec<Sort>>,
        pagination: Option<&Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let mut builder = QueryBuilder::new(table_name);

        if let Some(sorts) = sort {
            builder = builder.order_by_multiple(sorts);
        }

        let pagination = pagination.unwrap_or(&Pagination::default()).clone();
        builder.execute_paginated::<T>(db, &pagination).await
    }

    /// List records with filter, sorting, and pagination
    pub async fn list_where<T>(
        filter: FilterOperator,
        sort: Option<Vec<Sort>>,
        pagination: Option<&Pagination>,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        Self::list_where_with_table(filter, sort, pagination, db, T::table_name()).await
    }

    pub async fn list_where_with_table<T>(
        filter: FilterOperator,
        sort: Option<Vec<Sort>>,
        pagination: Option<&Pagination>,
        db: &Database,
        table_name: &str,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        let mut builder = QueryBuilder::new(table_name)._where(filter);

        if let Some(sorts) = sort {
            builder = builder.order_by_multiple(sorts);
        }

        let pagination = pagination.unwrap_or(&Pagination::default()).clone();
        builder.execute_paginated::<T>(db, &pagination).await
    }

    /// Execute a custom query
    pub async fn query<T>(builder: QueryBuilder, db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        builder.execute::<T>(db).await
    }

    /// Execute a custom query with table override
    pub async fn query_with_table<T>(builder: QueryBuilder, db: &Database) -> Result<Vec<T>>
    where
        T: crate::Orso,
    {
        // Note: QueryBuilder already accepts custom table names in constructor
        // This method is provided for API consistency
        builder.execute::<T>(db).await
    }

    /// Execute a custom query with pagination
    pub async fn query_paginated<T>(
        builder: QueryBuilder,
        pagination: &Pagination,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        builder.execute_paginated::<T>(db, pagination).await
    }

    /// Execute a custom query with pagination and table override
    pub async fn query_paginated_with_table<T>(
        builder: QueryBuilder,
        pagination: &Pagination,
        db: &Database,
    ) -> Result<PaginatedResult<T>>
    where
        T: crate::Orso,
    {
        // Note: QueryBuilder already accepts custom table names in constructor
        // This method is provided for API consistency
        builder.execute_paginated::<T>(db, pagination).await
    }

    /// Get aggregate value
    pub async fn aggregate<T>(
        function: Aggregate,
        column: &str,
        filter: Option<FilterOperator>,
        db: &Database,
    ) -> Result<Option<f64>>
    where
        T: crate::Orso,
    {
        Self::aggregate_with_table::<T>(function, column, filter, db, T::table_name()).await
    }

    pub async fn aggregate_with_table<T>(
        function: Aggregate,
        column: &str,
        filter: Option<FilterOperator>,
        db: &Database,
        table_name: &str,
    ) -> Result<Option<f64>>
    where
        T: crate::Orso,
    {
        let mut builder = QueryBuilder::new(table_name).aggregate(function, column, None::<String>);

        if let Some(filter) = filter {
            builder = builder._where(filter);
        }

        let (sql, params) = builder.build()?;

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Send + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();

        let rows = db.query(&sql, &param_refs).await?;

        if let Some(row) = rows.get(0) {
            // Try to get as f64 first, then as i64 and convert
            if let Ok(value) = row.try_get::<_, f64>(0) {
                Ok(Some(value))
            } else if let Ok(value) = row.try_get::<_, i64>(0) {
                Ok(Some(value as f64))
            } else {
                Err(Error::Query("Failed to get aggregate value".to_string()))
            }
        } else {
            Ok(None)
        }
    }

    /// Convert a database row to a HashMap
    pub fn row_to_map(row: &tokio_postgres::Row) -> Result<HashMap<String, crate::Value>> {
        let mut map = HashMap::new();
        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name();
            let value = crate::Value::from_postgres_row(row, i)?;
            map.insert(column_name.to_string(), value);
        }
        Ok(map)
    }
}

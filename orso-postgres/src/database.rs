use crate::{Error, Result};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use serde::{Deserialize, Serialize};
use tokio_postgres::{NoTls, Row};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub connection_string: String,
    pub max_pool_size: usize,
}

impl DatabaseConfig {
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            max_pool_size: 16,
        }
    }

    pub fn postgres(connection_string: impl Into<String>) -> Self {
        Self::new(connection_string)
    }

    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.max_pool_size = size;
        self
    }
}

#[derive(Debug)]
pub struct Database {
    pub pool: Pool,
}

impl Database {
    pub async fn init(config: DatabaseConfig) -> Result<Self> {
        let pg_config: tokio_postgres::Config = config
            .connection_string
            .parse()
            .map_err(|e| Error::Config(format!("Invalid connection string: {}", e)))?;

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
        let pool = Pool::builder(mgr)
            .max_size(config.max_pool_size)
            .build()
            .map_err(|e| Error::Connection(format!("Failed to create connection pool: {}", e)))?;

        debug!(
            "PostgreSQL connection pool established with max_size: {}",
            config.max_pool_size
        );

        Ok(Self { pool })
    }

    pub async fn execute(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Send + Sync)],
    ) -> Result<u64> {
        let client = self.pool.get().await?;

        // Convert Send + Sync to Sync at the boundary (secure coercion)
        let sync_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| *p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = client.execute(sql, &sync_params).await?;
        Ok(rows)
    }

    pub async fn query(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Send + Sync)],
    ) -> Result<Vec<Row>> {
        let client = self.pool.get().await?;

        // Convert Send + Sync to Sync at the boundary (secure coercion)
        let sync_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| *p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = client.query(sql, &sync_params).await?;
        Ok(rows)
    }

    pub async fn query_one(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Send + Sync)],
    ) -> Result<Row> {
        let client = self.pool.get().await?;

        // Convert Send + Sync to Sync at the boundary (secure coercion)
        let sync_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| *p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let row = client.query_one(sql, &sync_params).await?;
        Ok(row)
    }

    pub async fn query_opt(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Send + Sync)],
    ) -> Result<Option<Row>> {
        let client = self.pool.get().await?;

        // Convert Send + Sync to Sync at the boundary (secure coercion)
        let sync_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| *p as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let row = client.query_opt(sql, &sync_params).await?;
        Ok(row)
    }
}

use orso_postgres::{migration, Database, DatabaseConfig, Migrations, Orso};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
#[orso_table("test_manual_creation")]
struct TestManualCreation {
    #[orso_column(primary_key)]
    id: Option<String>,
    name: String,
    hello: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = DatabaseConfig::new("postgresql://postgres@localhost:1332/postgres".to_string());
    let db = Database::init(config).await?;

    println!("Creating table using migration system...");
    let results = Migrations::init(&db, &[migration!(TestManualCreation)]).await?;
    println!("Migration result: {:?}", results);

    println!("Generated migration SQL:");
    println!("{}", TestManualCreation::migration_sql());

    Ok(())
}

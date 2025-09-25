// Test database operations with OrsoDateTime to verify the timestamp fix
use orso_postgres::{migration, Database, DatabaseConfig, Migrations, Orso, OrsoDateTime};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
#[orso_table("test_timestamp_db")]
struct TimestampTest {
    #[orso_column(primary_key)]
    id: Option<String>,

    name: String,

    #[orso_column(created_at)]
    created_at: Option<OrsoDateTime>,

    #[orso_column(updated_at)]
    updated_at: Option<OrsoDateTime>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing OrsoDateTime Database Operations ===\n");

    // Create database connection
    let config = DatabaseConfig::new("postgresql://postgres@localhost:1332/postgres".to_string());
    let db = Database::init(config).await?;

    // Clean up any existing test data
    let _ = db
        .execute("DROP TABLE IF EXISTS test_timestamp_db CASCADE", &[])
        .await;

    // Initialize migrations
    Migrations::init(&db, &[migration!(TimestampTest)]).await?;
    println!("✓ Table created successfully");

    // Test 1: Insert with None timestamps (should auto-generate)
    let test_record = TimestampTest {
        id: None,
        name: "Test Record 1".to_string(),
        created_at: None,
        updated_at: None,
    };

    test_record.insert(&db).await?;
    println!("✓ Record inserted with None timestamps");

    // Test 2: Read back and verify timestamps were generated
    let records = TimestampTest::find_all(&db).await?;
    println!("✓ Found {} records", records.len());

    if let Some(record) = records.first() {
        println!("  - ID: {:?}", record.id);
        println!("  - Name: {}", record.name);
        println!("  - Created At: {:?}", record.created_at);
        println!("  - Updated At: {:?}", record.updated_at);

        if record.created_at.is_some() {
            println!("✓ created_at was auto-generated");
        } else {
            println!("✗ created_at was not generated");
        }
    }

    // Test 3: Insert with explicit timestamp
    let explicit_timestamp = OrsoDateTime::now();
    let test_record2 = TimestampTest {
        id: None,
        name: "Test Record 2".to_string(),
        created_at: Some(explicit_timestamp),
        updated_at: None,
    };

    test_record2.insert(&db).await?;
    println!("✓ Record inserted with explicit timestamp");

    // Test 4: Read all records and verify both timestamp scenarios
    let all_records = TimestampTest::find_all(&db).await?;
    println!("✓ Found {} total records", all_records.len());

    for (i, record) in all_records.iter().enumerate() {
        println!(
            "  Record {}: name='{}', created_at={:?}",
            i + 1,
            record.name,
            record.created_at
        );
    }

    // Test 5: Update operation to trigger updated_at
    if let Some(mut record) = all_records.first().cloned() {
        record.name = "Updated Name".to_string();
        record.update(&db).await?;
        println!("✓ Record updated");

        // Read back to verify updated_at was set
        let updated_records = TimestampTest::find_all(&db).await?;
        if let Some(updated_record) = updated_records.first() {
            if updated_record.updated_at.is_some() {
                println!("✓ updated_at was set during update");
                println!("  - Updated At: {:?}", updated_record.updated_at);
            } else {
                println!("✗ updated_at was not set");
            }
        }
    }

    // Test 6: Find by ID to test single record retrieval
    if let Some(record) = all_records.first() {
        if let Some(id) = &record.id {
            let found_record = TimestampTest::find_by_id(id, &db).await?;
            if let Some(found) = found_record {
                println!("✓ Successfully found record by ID");
                println!(
                    "  - Retrieved: name='{}', created_at={:?}",
                    found.name, found.created_at
                );
            } else {
                println!("✗ Could not find record by ID");
            }
        }
    }

    // Cleanup
    let _ = db
        .execute("DROP TABLE IF EXISTS test_timestamp_db CASCADE", &[])
        .await;
    println!("\n=== All timestamp database operations completed successfully! ===");

    Ok(())
}

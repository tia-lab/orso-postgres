// Test external usage like a real application would use it
use chrono::{DateTime, Utc};
use orso_postgres::{orso_column, Orso};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone, Orso)]
pub struct TestExternalUsage {
    #[orso_column(primary_key)]
    pub id: Option<String>,

    #[orso_column(updated_at)]
    pub updated_at: Option<DateTime<Utc>>,

    pub name: String,
    pub age: i32,
}

fn main() {
    println!("External usage test compiled successfully!");

    let test_struct = TestExternalUsage {
        id: None,
        updated_at: None,
        name: "Test".to_string(),
        age: 25,
    };

    println!("Test struct: {:?}", test_struct);
}

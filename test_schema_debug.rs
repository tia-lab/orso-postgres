use orso::{Orso};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
#[orso_table("debug_primary_key")]
struct DebugPrimaryKey {
    #[orso_column(primary_key)]
    id: Option<String>,
    name: String,
}

fn main() {
    println!("Migration SQL:");
    println!("{}", DebugPrimaryKey::migration_sql());
    println!("\nCreate table SQL:");
    println!("{}", DebugPrimaryKey::create_table_sql());
}
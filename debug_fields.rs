use orso_postgres::*;
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug, Default)]
struct DebugFields {
    #[orso_column(primary_key)]
    id: Option<String>,

    uncompressed: Vec<i64>,

    #[orso_column(compress)]
    compressed: Vec<i64>,

    name: String,
}

fn main() {
    let field_names = DebugFields::field_names();
    let field_types = DebugFields::field_types();
    let compressed_flags = DebugFields::field_compressed();

    println!("Field analysis:");
    for (i, name) in field_names.iter().enumerate() {
        println!("  {}: {} -> {:?} -> compressed: {}",
                i, name, field_types.get(i), compressed_flags.get(i).unwrap_or(&false));
    }

    // Test to_map behavior
    let test_record = DebugFields {
        id: None,
        uncompressed: vec![1, 2, 3],
        compressed: vec![4, 5, 6],
        name: "Test".to_string(),
    };

    match test_record.to_map() {
        Ok(map) => {
            println!("\nto_map results:");
            for (k, v) in &map {
                println!("  {}: {:?}", k, v);
            }
        }
        Err(e) => println!("to_map error: {:?}", e),
    }
}
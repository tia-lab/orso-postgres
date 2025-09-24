use chrono::Timelike;
use orso_postgres::{Orso, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Orso, Serialize, Deserialize, Clone, Debug)]
#[orso_table("test_datetime")]
struct TestDateTime {
    #[orso_column(primary_key)]
    id: Option<String>,

    name: String,

    // Using our DateTime wrapper
    my_timestamp: Timestamp,

    // Using chrono::DateTime directly
    my_chrono_date: chrono::DateTime<chrono::Utc>,

    #[orso_column(created_at)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,

    #[orso_column(updated_at)]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn validate_precise_date(date_str: &str, expected_date: &str) -> bool {
    // Parse the input date string
    match chrono::DateTime::parse_from_rfc3339(date_str) {
        Ok(parsed_date) => {
            let utc_date = parsed_date.with_timezone(&chrono::Utc);

            // Parse expected date
            if let Ok(expected) = chrono::DateTime::parse_from_rfc3339(expected_date) {
                let expected_utc = expected.with_timezone(&chrono::Utc);

                // Compare dates (you can adjust precision as needed)
                let diff = (utc_date.timestamp() - expected_utc.timestamp()).abs();
                println!("Comparing: {} vs {}", utc_date, expected_utc);
                println!("Time difference: {} seconds", diff);

                // Allow 1 second tolerance
                diff <= 1
            } else {
                println!("Failed to parse expected date: {}", expected_date);
                false
            }
        }
        Err(e) => {
            println!("Failed to parse date '{}': {}", date_str, e);
            false
        }
    }
}

fn validate_timestamp_wrapper(timestamp: &Timestamp, expected_date: &str) -> bool {
    // Get the inner DateTime
    let inner_dt = timestamp.inner();

    // Parse expected date
    match chrono::DateTime::parse_from_rfc3339(expected_date) {
        Ok(expected) => {
            let expected_utc = expected.with_timezone(&chrono::Utc);
            let diff = (inner_dt.timestamp() - expected_utc.timestamp()).abs();

            println!("Timestamp validation: {} vs {}", inner_dt, expected_utc);
            println!("Difference: {} seconds", diff);

            diff <= 1 // 1 second tolerance
        }
        Err(e) => {
            println!("Failed to parse expected date: {}", e);
            false
        }
    }
}

fn create_precise_timestamp(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: u32,
) -> Option<Timestamp> {
    // Create precise datetime
    let naive_dt = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|d| d.and_hms_opt(hour, min, sec))?;

    let utc_dt = chrono::DateTime::from_naive_utc_and_offset(naive_dt, chrono::Utc);
    Some(Timestamp::new(utc_dt))
}

fn main() {
    println!("=== DateTime Validation Tests ===\n");

    // Test 1: Basic DateTime wrapper
    let now = chrono::Utc::now();
    let timestamp = Timestamp::new(now);

    println!("1. Basic DateTime wrapper:");
    println!("   Original DateTime: {:?}", now);
    println!("   Timestamp wrapper: {:?}", timestamp);
    println!(
        "   Serialized: {}\n",
        serde_json::to_string(&timestamp).unwrap()
    );

    // Test 2: Validate precise dates
    println!("2. Precise Date Validation:");
    let test_date = "2024-12-25T15:30:45Z";
    let expected_date = "2024-12-25T15:30:45Z";
    let is_valid = validate_precise_date(test_date, expected_date);
    println!("   Date '{}' is valid: {}\n", test_date, is_valid);

    // Test 3: Create and validate precise timestamp
    println!("3. Precise Timestamp Creation:");
    match create_precise_timestamp(2024, 12, 25, 15, 30, 45) {
        Some(precise_ts) => {
            println!("   Created precise timestamp: {:?}", precise_ts);
            let is_valid = validate_timestamp_wrapper(&precise_ts, "2024-12-25T15:30:45Z");
            println!("   Validation result: {}\n", is_valid);

            // Show how to use it in a struct
            let test_data = TestDateTime {
                id: Some("precise-test".to_string()),
                name: "Precise Date Test".to_string(),
                my_timestamp: precise_ts,
                my_chrono_date: chrono::Utc::now(),
                created_at: None,
                updated_at: None,
            };
            println!("   Test struct with precise date: {:?}\n", test_data);
        }
        None => println!("   Failed to create precise timestamp: Invalid date/time\n"),
    }

    // Test 4: Date range validation
    println!("4. Date Range Validation:");
    let start_date = "2024-01-01T00:00:00Z";
    let end_date = "2024-12-31T23:59:59Z";
    let test_dates = vec![
        "2024-06-15T12:00:00Z", // Valid - within range
        "2023-12-31T23:59:59Z", // Invalid - before range
        "2025-01-01T00:00:00Z", // Invalid - after range
    ];

    for date in &test_dates {
        let is_in_range = validate_date_range(date, start_date, end_date);
        println!("   Date '{}' is in range: {}", date, is_in_range);
    }
    println!();

    // Test 5: Business hours validation
    println!("5. Business Hours Validation:");
    let business_start = 9; // 9 AM
    let business_end = 17; // 5 PM
    let test_times = vec![
        "2024-06-15T10:30:00Z", // Valid business hour
        "2024-06-15T06:30:00Z", // Too early
        "2024-06-15T19:30:00Z", // Too late
    ];

    for time in &test_times {
        let is_business_hours = validate_business_hours(time, business_start, business_end);
        println!(
            "   Time '{}' is business hours: {}",
            time, is_business_hours
        );
    }
    println!();

    // Test 6: Serialization/deserialization validation
    println!("6. Serialization/Deserialization Test:");
    let original_ts = Timestamp::now();
    match serde_json::to_string(&original_ts) {
        Ok(json) => {
            println!("   Serialized: {}", json);
            match serde_json::from_str::<Timestamp>(&json) {
                Ok(deserialized_ts) => {
                    let diff = (original_ts.inner().timestamp()
                        - deserialized_ts.inner().timestamp())
                    .abs();
                    println!("   Deserialized successfully, difference: {} seconds", diff);
                    println!("   Round-trip successful: {}", diff == 0);
                }
                Err(e) => println!("   Deserialization failed: {}", e),
            }
        }
        Err(e) => println!("   Serialization failed: {}", e),
    }

    println!("\n=== DateTime implementation complete! ===");
}

fn validate_date_range(date_str: &str, start_str: &str, end_str: &str) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(date_str),
        chrono::DateTime::parse_from_rfc3339(start_str),
        chrono::DateTime::parse_from_rfc3339(end_str),
    ) {
        (Ok(date), Ok(start), Ok(end)) => {
            let date_utc = date.with_timezone(&chrono::Utc);
            let start_utc = start.with_timezone(&chrono::Utc);
            let end_utc = end.with_timezone(&chrono::Utc);

            date_utc >= start_utc && date_utc <= end_utc
        }
        _ => false,
    }
}

fn validate_business_hours(date_str: &str, start_hour: u32, end_hour: u32) -> bool {
    match chrono::DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => {
            let hour = dt.hour();
            hour >= start_hour && hour < end_hour
        }
        Err(_) => false,
    }
}

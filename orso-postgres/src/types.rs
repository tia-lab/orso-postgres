use serde::{Deserialize, Deserializer, Serialize, Serializer};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Boolean(bool),
    DateTime(DateTime<Utc>),
    // Array types for PostgreSQL native arrays
    IntegerArray(Vec<i32>),    // INTEGER[] - for i32, i16, i8, u32, u16, u8
    BigIntArray(Vec<i64>),     // BIGINT[] - for i64, u64
    NumericArray(Vec<f64>),    // DOUBLE PRECISION[] - for f64, f32
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Integer(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Real(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_string())
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Boolean(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Blob(v)
    }
}

impl From<Option<String>> for Value {
    fn from(v: Option<String>) -> Self {
        match v {
            Some(s) => Value::Text(s),
            None => Value::Null,
        }
    }
}

impl From<Option<i64>> for Value {
    fn from(v: Option<i64>) -> Self {
        match v {
            Some(i) => Value::Integer(i),
            None => Value::Null,
        }
    }
}

impl From<Option<f64>> for Value {
    fn from(v: Option<f64>) -> Self {
        match v {
            Some(f) => Value::Real(f),
            None => Value::Null,
        }
    }
}

impl From<Option<bool>> for Value {
    fn from(v: Option<bool>) -> Self {
        match v {
            Some(b) => Value::Boolean(b),
            None => Value::Null,
        }
    }
}

impl From<Option<Vec<u8>>> for Value {
    fn from(v: Option<Vec<u8>>) -> Self {
        match v {
            Some(b) => Value::Blob(b),
            None => Value::Null,
        }
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(v: DateTime<Utc>) -> Self {
        Value::DateTime(v)
    }
}

impl From<Option<DateTime<Utc>>> for Value {
    fn from(v: Option<DateTime<Utc>>) -> Self {
        match v {
            Some(dt) => Value::DateTime(dt),
            None => Value::Null,
        }
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Boolean(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Real(f)
                } else {
                    Value::Text(n.to_string())
                }
            }
            serde_json::Value::String(s) => Value::Text(s),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                Value::Text(v.to_string())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::Asc => write!(f, "ASC"),
            SortOrder::Desc => write!(f, "DESC"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Aggregate {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl std::fmt::Display for Aggregate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Aggregate::Count => write!(f, "COUNT"),
            Aggregate::Sum => write!(f, "SUM"),
            Aggregate::Avg => write!(f, "AVG"),
            Aggregate::Min => write!(f, "MIN"),
            Aggregate::Max => write!(f, "MAX"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl std::fmt::Display for JoinType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinType::Inner => write!(f, "INNER JOIN"),
            JoinType::Left => write!(f, "LEFT JOIN"),
            JoinType::Right => write!(f, "RIGHT JOIN"),
            JoinType::Full => write!(f, "FULL JOIN"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Operator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Like,
    NotLike,
    In,
    NotIn,
    IsNull,
    IsNotNull,
    Between,
    NotBetween,
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Eq => write!(f, "="),
            Operator::Ne => write!(f, "!="),
            Operator::Lt => write!(f, "<"),
            Operator::Le => write!(f, "<="),
            Operator::Gt => write!(f, ">"),
            Operator::Ge => write!(f, ">="),
            Operator::Like => write!(f, "LIKE"),
            Operator::NotLike => write!(f, "NOT LIKE"),
            Operator::In => write!(f, "IN"),
            Operator::NotIn => write!(f, "NOT IN"),
            Operator::IsNull => write!(f, "IS NULL"),
            Operator::IsNotNull => write!(f, "IS NOT NULL"),
            Operator::Between => write!(f, "BETWEEN"),
            Operator::NotBetween => write!(f, "NOT BETWEEN"),
        }
    }
}

impl Value {
    pub fn to_postgres_param(&self) -> Box<dyn tokio_postgres::types::ToSql + Send + Sync> {
        match self {
            Value::Null => Box::new(Option::<String>::None),
            Value::Integer(i) => {
                // Check if the value fits in i32 range for PostgreSQL INTEGER columns
                if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 {
                    // Use i32 to ensure compatibility with INTEGER columns
                    Box::new(*i as i32)
                } else {
                    // Use i64 for BIGINT columns
                    Box::new(*i)
                }
            }
            Value::Real(f) => Box::new(*f),
            Value::Text(s) => Box::new(s.clone()),
            Value::DateTime(dt) => {
                // Convert DateTime directly to SystemTime for PostgreSQL
                Box::new(std::time::SystemTime::from(*dt))
            },
            Value::Blob(b) => Box::new(b.clone()),
            Value::Boolean(b) => Box::new(*b),
            // Array types - pass directly to PostgreSQL
            Value::IntegerArray(arr) => Box::new(arr.clone()),
            Value::BigIntArray(arr) => Box::new(arr.clone()),
            Value::NumericArray(arr) => Box::new(arr.clone()),
        }
    }

    pub fn from_postgres_row(row: &tokio_postgres::Row, idx: usize) -> crate::Result<Self> {
        let column = &row.columns()[idx];
        let type_name = column.type_().name();

        match type_name {
            "int8" | "bigint" => {
                let val: Option<i64> = row.try_get(idx)?;
                Ok(val.map(Value::Integer).unwrap_or(Value::Null))
            }
            "int4" | "integer" => {
                let val: Option<i32> = row.try_get(idx)?;
                Ok(val.map(|i| Value::Integer(i as i64)).unwrap_or(Value::Null))
            }
            "float8" | "double precision" => {
                let val: Option<f64> = row.try_get(idx)?;
                Ok(val.map(Value::Real).unwrap_or(Value::Null))
            }
            "text" | "varchar" => {
                let val: Option<String> = row.try_get(idx)?;
                Ok(val.map(Value::Text).unwrap_or(Value::Null))
            }
            "bytea" => {
                let val: Option<Vec<u8>> = row.try_get(idx)?;
                Ok(val.map(Value::Blob).unwrap_or(Value::Null))
            }
            "bool" | "boolean" => {
                let val: Option<bool> = row.try_get(idx)?;
                Ok(val.map(Value::Boolean).unwrap_or(Value::Null))
            }
            "timestamp" | "timestamptz" => {
                // Handle PostgreSQL timestamps using SystemTime, convert to DateTime
                let val: Option<std::time::SystemTime> = row.try_get(idx)?;
                Ok(val
                    .map(|st| {
                        let datetime = chrono::DateTime::<chrono::Utc>::from(st);
                        Value::DateTime(datetime)
                    })
                    .unwrap_or(Value::Null))
            }
            "_int8" | "int8[]" => {
                // PostgreSQL BIGINT array
                let val: Option<Vec<i64>> = row.try_get(idx)?;
                Ok(val.map(Value::BigIntArray).unwrap_or(Value::Null))
            }
            "_int4" | "int4[]" => {
                // PostgreSQL INTEGER array
                let val: Option<Vec<i32>> = row.try_get(idx)?;
                Ok(val.map(Value::IntegerArray).unwrap_or(Value::Null))
            }
            "_float8" | "float8[]" => {
                // PostgreSQL DOUBLE PRECISION array
                let val: Option<Vec<f64>> = row.try_get(idx)?;
                Ok(val.map(Value::NumericArray).unwrap_or(Value::Null))
            }
            _ => {
                // Try as string for unknown types
                let val: Option<String> = row.try_get(idx)?;
                Ok(val.map(Value::Text).unwrap_or(Value::Null))
            }
        }
    }
}

/// DateTime wrapper that ensures consistent PostgreSQL timestamp handling
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub DateTime<Utc>);

impl Timestamp {
    pub fn new(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }

    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn inner(&self) -> &DateTime<Utc> {
        &self.0
    }

    pub fn into_inner(self) -> DateTime<Utc> {
        self.0
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl std::ops::Deref for Timestamp {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Always use PostgreSQL format for serialization
        let formatted = crate::Utils::create_timestamp(self.0);
        serializer.serialize_str(&formatted)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let s = String::deserialize(deserializer)?;
        crate::Utils::parse_timestamp(&s)
            .map(Timestamp)
            .map_err(|e| Error::custom(format!("Invalid timestamp format: {}", e)))
    }
}

impl From<Timestamp> for Value {
    fn from(ts: Timestamp) -> Self {
        Value::DateTime(ts.0)
    }
}

impl From<Option<Timestamp>> for Value {
    fn from(ts: Option<Timestamp>) -> Self {
        match ts {
            Some(t) => Value::DateTime(t.0),
            None => Value::Null,
        }
    }
}

pub fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i != 0)
            } else if let Some(f) = n.as_f64() {
                Ok(f != 0.0)
            } else {
                Err(Error::custom("Invalid number format for boolean"))
            }
        }
        serde_json::Value::String(s) => match s.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(Error::custom(format!(
                "Invalid string value for boolean: {s}"
            ))),
        },
        _ => Err(Error::custom("Expected boolean, integer, or string")),
    }
}

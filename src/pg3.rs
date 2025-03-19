//! Postgres relational database storage.
//! 
//! You can use the [`into()`](std::convert::Into) method to convert
//! a Rust value into a [`ParameterValue`]. You can use the
//! [`Decode`] trait to convert a [`DbValue`] to a suitable Rust type.
//! The following table shows available conversions.
//!
//! # Types
//!
//! | Rust type               | WIT (db-value)                                | Postgres type(s)             |
//! |-------------------------|-----------------------------------------------|----------------------------- |
//! | `bool`                  | boolean(bool)                                 | BOOL                         |
//! | `i16`                   | int16(s16)                                    | SMALLINT, SMALLSERIAL, INT2  |
//! | `i32`                   | int32(s32)                                    | INT, SERIAL, INT4            |
//! | `i64`                   | int64(s64)                                    | BIGINT, BIGSERIAL, INT8      |
//! | `f32`                   | floating32(float32)                           | REAL, FLOAT4                 |
//! | `f64`                   | floating64(float64)                           | DOUBLE PRECISION, FLOAT8     |
//! | `String`                | str(string)                                   | VARCHAR, CHAR(N), TEXT       |
//! | `Vec<u8>`               | binary(list\<u8\>)                            | BYTEA                        |
//! | `chrono::NaiveDate`     | date(tuple<s32, u8, u8>)                      | DATE                         |
//! | `chrono::NaiveTime`     | time(tuple<u8, u8, u8, u32>)                  | TIME                         |
//! | `chrono::NaiveDateTime` | datetime(tuple<s32, u8, u8, u8, u8, u8, u32>) | TIMESTAMP                    |
//! | `chrono::Duration`      | timestamp(s64)                                | BIGINT                       |

/// An open connection to a PostgreSQL database.
/// 
/// # Examples
/// 
/// Load a set of rows from a local PostgreSQL database, and iterate over them.
/// 
/// ```no_run
/// use spin_sdk::pg3::{Connection, Decode};
///
/// # fn main() -> anyhow::Result<()> {
/// # let min_age = 0;
/// let db = Connection::open("host=localhost user=postgres password=my_password dbname=mydb")?;
///
/// let query_result = db.query(
///     "SELECT * FROM users WHERE age >= $1",
///     &[min_age.into()]
/// )?;
///
/// let name_index = query_result.columns.iter().position(|c| c.name == "name").unwrap();
///
/// for row in &query_result.rows {
///     let name = String::decode(&row[name_index])?;
///     println!("Found user {name}");
/// }
/// # Ok(())
/// # }
/// ```
/// 
/// Perform an aggregate (scalar) operation over a table. The result set
/// contains a single column, with a single row.
/// 
/// ```no_run
/// use spin_sdk::pg3::{Connection, Decode};
///
/// # fn main() -> anyhow::Result<()> {
/// let db = Connection::open("host=localhost user=postgres password=my_password dbname=mydb")?;
///
/// let query_result = db.query("SELECT COUNT(*) FROM users", &[])?;
///
/// assert_eq!(1, query_result.columns.len());
/// assert_eq!("count", query_result.columns[0].name);
/// assert_eq!(1, query_result.rows.len());
///
/// let count = i64::decode(&query_result.rows[0][0])?;
/// # Ok(())
/// # }
/// ```
/// 
/// Delete rows from a PostgreSQL table. This uses [Connection::execute()]
/// instead of the `query` method.
///
/// ```no_run
/// use spin_sdk::pg3::Connection;
///
/// # fn main() -> anyhow::Result<()> {
/// let db = Connection::open("host=localhost user=postgres password=my_password dbname=mydb")?;
///
/// let rows_affected = db.execute(
///     "DELETE FROM users WHERE name = $1",
///     &["Baldrick".to_owned().into()]
/// )?;
/// # Ok(())
/// # }
/// ```
#[doc(inline)]
pub use super::wit::pg3::Connection;

/// The result of a database query.
///
/// # Examples
/// 
/// Load a set of rows from a local PostgreSQL database, and iterate over them
/// selecting one field from each. The columns collection allows you to find
/// column indexes for column names; you can bypass this lookup if you name
/// specific columns in the query.
/// 
/// ```no_run
/// use spin_sdk::pg3::{Connection, Decode};
///
/// # fn main() -> anyhow::Result<()> {
/// # let min_age = 0;
/// let db = Connection::open("host=localhost user=postgres password=my_password dbname=mydb")?;
///
/// let query_result = db.query(
///     "SELECT * FROM users WHERE age >= $1",
///     &[min_age.into()]
/// )?;
///
/// let name_index = query_result.columns.iter().position(|c| c.name == "name").unwrap();
///
/// for row in &query_result.rows {
///     let name = String::decode(&row[name_index])?;
///     println!("Found user {name}");
/// }
/// # Ok(())
/// # }
/// ```
pub use super::wit::pg3::RowSet;

#[doc(inline)]
pub use super::wit::pg3::{Error as PgError, *};

use chrono::{Datelike, Timelike};

/// A Postgres error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to deserialize [`DbValue`]
    #[error("error value decoding: {0}")]
    Decode(String),
    /// Postgres query failed with an error
    #[error(transparent)]
    PgError(#[from] PgError),
}

/// A type that can be decoded from the database.
pub trait Decode: Sized {
    /// Decode a new value of this type using a [`DbValue`].
    fn decode(value: &DbValue) -> Result<Self, Error>;
}

impl<T> Decode for Option<T>
where
    T: Decode,
{
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::DbNull => Ok(None),
            v => Ok(Some(T::decode(v)?)),
        }
    }
}

impl Decode for bool {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Boolean(boolean) => Ok(*boolean),
            _ => Err(Error::Decode(format_decode_err("BOOL", value))),
        }
    }
}

impl Decode for i16 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Int16(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("SMALLINT", value))),
        }
    }
}

impl Decode for i32 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Int32(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("INT", value))),
        }
    }
}

impl Decode for i64 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Int64(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("BIGINT", value))),
        }
    }
}

impl Decode for f32 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Floating32(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("REAL", value))),
        }
    }
}

impl Decode for f64 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Floating64(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("DOUBLE PRECISION", value))),
        }
    }
}

impl Decode for Vec<u8> {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Binary(n) => Ok(n.to_owned()),
            _ => Err(Error::Decode(format_decode_err("BYTEA", value))),
        }
    }
}

impl Decode for String {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Str(s) => Ok(s.to_owned()),
            _ => Err(Error::Decode(format_decode_err(
                "CHAR, VARCHAR, TEXT",
                value,
            ))),
        }
    }
}

impl Decode for chrono::NaiveDate {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Date((year, month, day)) => {
                let naive_date =
                    chrono::NaiveDate::from_ymd_opt(*year, (*month).into(), (*day).into())
                        .ok_or_else(|| {
                            Error::Decode(format!(
                                "invalid date y={}, m={}, d={}",
                                year, month, day
                            ))
                        })?;
                Ok(naive_date)
            }
            _ => Err(Error::Decode(format_decode_err("DATE", value))),
        }
    }
}

impl Decode for chrono::NaiveTime {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Time((hour, minute, second, nanosecond)) => {
                let naive_time = chrono::NaiveTime::from_hms_nano_opt(
                    (*hour).into(),
                    (*minute).into(),
                    (*second).into(),
                    *nanosecond,
                )
                .ok_or_else(|| {
                    Error::Decode(format!(
                        "invalid time {}:{}:{}:{}",
                        hour, minute, second, nanosecond
                    ))
                })?;
                Ok(naive_time)
            }
            _ => Err(Error::Decode(format_decode_err("TIME", value))),
        }
    }
}

impl Decode for chrono::NaiveDateTime {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Datetime((year, month, day, hour, minute, second, nanosecond)) => {
                let naive_date =
                    chrono::NaiveDate::from_ymd_opt(*year, (*month).into(), (*day).into())
                        .ok_or_else(|| {
                            Error::Decode(format!(
                                "invalid date y={}, m={}, d={}",
                                year, month, day
                            ))
                        })?;
                let naive_time = chrono::NaiveTime::from_hms_nano_opt(
                    (*hour).into(),
                    (*minute).into(),
                    (*second).into(),
                    *nanosecond,
                )
                .ok_or_else(|| {
                    Error::Decode(format!(
                        "invalid time {}:{}:{}:{}",
                        hour, minute, second, nanosecond
                    ))
                })?;
                let dt = chrono::NaiveDateTime::new(naive_date, naive_time);
                Ok(dt)
            }
            _ => Err(Error::Decode(format_decode_err("DATETIME", value))),
        }
    }
}

impl Decode for chrono::Duration {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Timestamp(n) => Ok(chrono::Duration::seconds(*n)),
            _ => Err(Error::Decode(format_decode_err("BIGINT", value))),
        }
    }
}

macro_rules! impl_parameter_value_conversions {
    ($($ty:ty => $id:ident),*) => {
        $(
            impl From<$ty> for ParameterValue {
                fn from(v: $ty) -> ParameterValue {
                    ParameterValue::$id(v)
                }
            }
        )*
    };
}

impl_parameter_value_conversions! {
    i8 => Int8,
    i16 => Int16,
    i32 => Int32,
    i64 => Int64,
    f32 => Floating32,
    f64 => Floating64,
    bool => Boolean,
    String => Str,
    Vec<u8> => Binary
}

impl From<chrono::NaiveDateTime> for ParameterValue {
    fn from(v: chrono::NaiveDateTime) -> ParameterValue {
        ParameterValue::Datetime((
            v.year(),
            v.month() as u8,
            v.day() as u8,
            v.hour() as u8,
            v.minute() as u8,
            v.second() as u8,
            v.nanosecond(),
        ))
    }
}

impl From<chrono::NaiveTime> for ParameterValue {
    fn from(v: chrono::NaiveTime) -> ParameterValue {
        ParameterValue::Time((
            v.hour() as u8,
            v.minute() as u8,
            v.second() as u8,
            v.nanosecond(),
        ))
    }
}

impl From<chrono::NaiveDate> for ParameterValue {
    fn from(v: chrono::NaiveDate) -> ParameterValue {
        ParameterValue::Date((v.year(), v.month() as u8, v.day() as u8))
    }
}

impl From<chrono::TimeDelta> for ParameterValue {
    fn from(v: chrono::TimeDelta) -> ParameterValue {
        ParameterValue::Timestamp(v.num_seconds())
    }
}

impl<T: Into<ParameterValue>> From<Option<T>> for ParameterValue {
    fn from(o: Option<T>) -> ParameterValue {
        match o {
            Some(v) => v.into(),
            None => ParameterValue::DbNull,
        }
    }
}

fn format_decode_err(types: &str, value: &DbValue) -> String {
    format!("Expected {} from the DB but got {:?}", types, value)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    use super::*;

    #[test]
    fn boolean() {
        assert!(bool::decode(&DbValue::Boolean(true)).unwrap());
        assert!(bool::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<bool>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn int16() {
        assert_eq!(i16::decode(&DbValue::Int16(0)).unwrap(), 0);
        assert!(i16::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<i16>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn int32() {
        assert_eq!(i32::decode(&DbValue::Int32(0)).unwrap(), 0);
        assert!(i32::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<i32>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn int64() {
        assert_eq!(i64::decode(&DbValue::Int64(0)).unwrap(), 0);
        assert!(i64::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<i64>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn floating32() {
        assert!(f32::decode(&DbValue::Floating32(0.0)).is_ok());
        assert!(f32::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<f32>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn floating64() {
        assert!(f64::decode(&DbValue::Floating64(0.0)).is_ok());
        assert!(f64::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<f64>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn str() {
        assert_eq!(
            String::decode(&DbValue::Str(String::from("foo"))).unwrap(),
            String::from("foo")
        );

        assert!(String::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<String>::decode(&DbValue::DbNull)
            .unwrap()
            .is_none());
    }

    #[test]
    fn binary() {
        assert!(Vec::<u8>::decode(&DbValue::Binary(vec![0, 0])).is_ok());
        assert!(Vec::<u8>::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<Vec<u8>>::decode(&DbValue::DbNull)
            .unwrap()
            .is_none());
    }

    #[test]
    fn date() {
        assert_eq!(
            chrono::NaiveDate::decode(&DbValue::Date((1, 2, 4))).unwrap(),
            chrono::NaiveDate::from_ymd_opt(1, 2, 4).unwrap()
        );
        assert_ne!(
            chrono::NaiveDate::decode(&DbValue::Date((1, 2, 4))).unwrap(),
            chrono::NaiveDate::from_ymd_opt(1, 2, 5).unwrap()
        );
        assert!(Option::<chrono::NaiveDate>::decode(&DbValue::DbNull)
            .unwrap()
            .is_none());
    }

    #[test]
    fn time() {
        assert_eq!(
            chrono::NaiveTime::decode(&DbValue::Time((1, 2, 3, 4))).unwrap(),
            chrono::NaiveTime::from_hms_nano_opt(1, 2, 3, 4).unwrap()
        );
        assert_ne!(
            chrono::NaiveTime::decode(&DbValue::Time((1, 2, 3, 4))).unwrap(),
            chrono::NaiveTime::from_hms_nano_opt(1, 2, 4, 5).unwrap()
        );
        assert!(Option::<chrono::NaiveTime>::decode(&DbValue::DbNull)
            .unwrap()
            .is_none());
    }

    #[test]
    fn datetime() {
        let date = chrono::NaiveDate::from_ymd_opt(1, 2, 3).unwrap();
        let mut time = chrono::NaiveTime::from_hms_nano_opt(4, 5, 6, 7).unwrap();
        assert_eq!(
            chrono::NaiveDateTime::decode(&DbValue::Datetime((1, 2, 3, 4, 5, 6, 7))).unwrap(),
            chrono::NaiveDateTime::new(date, time)
        );

        time = chrono::NaiveTime::from_hms_nano_opt(4, 5, 6, 8).unwrap();
        assert_ne!(
            NaiveDateTime::decode(&DbValue::Datetime((1, 2, 3, 4, 5, 6, 7))).unwrap(),
            chrono::NaiveDateTime::new(date, time)
        );
        assert!(Option::<chrono::NaiveDateTime>::decode(&DbValue::DbNull)
            .unwrap()
            .is_none());
    }

    #[test]
    fn timestamp() {
        assert_eq!(
            chrono::Duration::decode(&DbValue::Timestamp(1)).unwrap(),
            chrono::Duration::seconds(1),
        );
        assert_ne!(
            chrono::Duration::decode(&DbValue::Timestamp(2)).unwrap(),
            chrono::Duration::seconds(1)
        );
        assert!(Option::<chrono::Duration>::decode(&DbValue::DbNull)
            .unwrap()
            .is_none());
    }
}

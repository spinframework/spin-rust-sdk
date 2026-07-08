//! MySQL relational database storage.
//!
//! You can use the [`Decode`] trait to convert a [`DbValue`] to a
//! suitable Rust type. The following table shows available conversions.
//!
//! # Types
//!
//! | Rust type | WIT (db-value)      | MySQL type(s)           |
//! |-----------|---------------------|-------------------------|
//! | `bool`    | int8(s8)            | TINYINT(1), BOOLEAN     |
//! | `i8`      | int8(s8)            | TINYINT                 |
//! | `i16`     | int16(s16)          | SMALLINT                |
//! | `i32`     | int32(s32)          | MEDIUM, INT             |
//! | `i64`     | int64(s64)          | BIGINT                  |
//! | `u8`      | uint8(u8)           | TINYINT UNSIGNED        |
//! | `u16`     | uint16(u16)         | SMALLINT UNSIGNED       |
//! | `u32`     | uint32(u32)         | INT UNSIGNED            |
//! | `u64`     | uint64(u64)         | BIGINT UNSIGNED         |
//! | `f32`     | floating32(float32) | FLOAT                   |
//! | `f64`     | floating64(float64) | DOUBLE                  |
//! | `String`  | str(string)         | VARCHAR, CHAR, TEXT     |
//! | `Vec<u8>` | binary(list\<u8\>)  | VARBINARY, BINARY, BLOB |

use crate::wit_bindgen;
use std::sync::Arc;

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]
    use crate::wit_bindgen;

    wit_bindgen::generate!({
        runtime_path: "crate::wit_bindgen::rt",
        world: "spin-sdk-mysql-v3",
        path: "wit",
        generate_all,
    });

    pub use spin::mysql::mysql;
}

/// An open connection to a MySQL database.
///
/// # Examples
///
/// Load a set of rows from a local MySQL database, and iterate over them.
///
/// ```no_run
/// use spin_sdk::mysql::{Connection, Decode, ParameterValue};
///
/// # async fn run() -> anyhow::Result<()> {
/// # let min_age = 0;
/// let db = Connection::open("mysql://root:my_password@localhost/mydb").await?;
///
/// let mut query_result = db.query(
///     "SELECT * FROM users WHERE age >= ?",
///     &[min_age.into()]
/// ).await?;
///
/// while let Some(row) = query_result.next().await {
///     let name = row.get::<String>("name").unwrap();
///     println!("Found user {name}");
/// }
///
/// query_result.result().await?;
/// # Ok(())
/// # }
/// ```
///
/// Perform an aggregate (scalar) operation over a table. The result set
/// contains a single column, with a single row.
///
/// ```no_run
/// use spin_sdk::mysql::{Connection, Decode};
///
/// # async fn run() -> anyhow::Result<()> {
/// let db = Connection::open("mysql://root:my_password@localhost/mydb").await?;
///
/// let mut query_result = db.query("SELECT COUNT(*) FROM users", &[]).await?;
///
/// assert_eq!(1, query_result.columns().len());
/// assert_eq!("COUNT(*)", query_result.columns()[0].name);
///
/// let rows = query_result.collect().await?;
///
/// assert_eq!(1, rows.len());
///
/// let count = &rows[0][0];
/// # Ok(())
/// # }
/// ```
///
/// Delete rows from a MySQL table. This uses [Connection::execute()]
/// instead of the `query` method.
///
/// ```no_run
/// use spin_sdk::mysql::{Connection, ParameterValue};
///
/// # async fn run() -> anyhow::Result<()> {
/// let db = Connection::open("mysql://root:my_password@localhost/mydb").await?;
///
/// db.execute(
///     "DELETE FROM users WHERE name = ?",
///     &["Baldrick".to_owned().into()]
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct Connection(wit::mysql::Connection);

impl Connection {
    /// Open a connection to a MySQL database.
    ///
    /// The address may be in connection string form (`"host=... dbname=..."`)
    /// or in URL form (`"mysql://<host>/<dbname>?..."`).
    pub async fn open(address: impl Into<String>) -> Result<Self, Error> {
        let inner = wit::mysql::Connection::open(address.into()).await?;
        Ok(Self(inner))
    }

    /// Query the database.
    ///
    /// Use this function for queries that return rows (typically `SELECT` queries).
    /// For side-effectful queries, see [`Connection::execute`].
    pub async fn query(
        &self,
        statement: impl Into<String>,
        params: impl Into<Vec<ParameterValue>>,
    ) -> Result<QueryResult, Error> {
        let (columns, rows, result) = self.0.query(statement.into(), params.into()).await?;
        Ok(QueryResult {
            columns: Arc::new(columns),
            rows,
            result,
        })
    }

    /// Execute a command against the database.
    ///
    /// Use this function for side-effectful queries (such as `INSERT` or `DELETE` queries).
    /// For queries that return row data, see [`Connection::query`].
    pub async fn execute(
        &self,
        statement: impl Into<String>,
        params: impl Into<Vec<ParameterValue>>,
    ) -> Result<(), Error> {
        self.0
            .execute(statement.into(), params.into())
            .await
            .map_err(Error::MysqlError)
    }
}

#[doc(inline)]
pub use wit::mysql::Error as MysqlError;

#[doc(inline)]
pub use wit::mysql::{Column, DbDataType, DbValue, ParameterValue};

/// The result of a [`Connection::query`] operation.
pub struct QueryResult {
    columns: Arc<Vec<Column>>,
    rows: wit_bindgen::StreamReader<Vec<DbValue>>,
    result: wit_bindgen::FutureReader<Result<(), MysqlError>>,
}

impl QueryResult {
    /// The columns in the query result.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Gets the next row in the result set.
    ///
    /// If this is `None`, there are no more rows available. You _must_
    /// await [`QueryResult::result()`] to determine if all rows
    /// were read successfully.
    pub async fn next(&mut self) -> Option<Row> {
        self.rows.next().await.map(|r| Row {
            columns: self.columns.clone(),
            result: r,
        })
    }

    /// Whether the query completed successfully or with an error.
    pub async fn result(self) -> Result<(), Error> {
        self.result.await.map_err(Error::MysqlError)
    }

    /// Collect all rows in the result set.
    ///
    /// This is provided for when the result set is small enough to fit in
    /// memory and you do not require streaming behaviour.
    pub async fn collect(mut self) -> Result<Vec<Row>, Error> {
        let mut rows = vec![];
        while let Some(row) = self.next().await {
            rows.push(row);
        }
        self.result.await.map_err(Error::MysqlError)?;
        Ok(rows)
    }

    /// An asynchronous reader for the rows of the query result. Call
    /// `.next().await` to iterate over the rows. When this returns `None`,
    /// you have read all available rows. At this point you _must_ check
    /// [`QueryResult::result()`] to determine if the read completed
    /// successfully.
    ///
    /// This provides each row as a plain vector of database values.
    /// [`QueryResult::next()`] provides a more ergonomic wrapper.
    ///
    /// To collect all rows into a vector, see [`QueryResult::collect`].
    pub fn rows(&mut self) -> &mut wit_bindgen::StreamReader<Vec<DbValue>> {
        &mut self.rows
    }

    /// Extracts the underlying Wasm Component Model results of the query.
    #[allow(
        clippy::type_complexity,
        reason = "sorry clippy that's just what the inner bits are"
    )]
    pub fn into_inner(
        self,
    ) -> (
        Vec<Column>,
        wit_bindgen::StreamReader<Vec<DbValue>>,
        wit_bindgen::FutureReader<Result<(), MysqlError>>,
    ) {
        ((*self.columns).clone(), self.rows, self.result)
    }
}

/// A database row result.
///
/// There are two representations of a MySQL row in the SDK.  This type is useful for
/// addressing elements by column name, and is obtained from the [QueryResult::next()] function.
/// The [DbValue] vector representation is obtained from the [QueryResult::rows()] function, and provides
/// index-based lookup or low-level access to row values via a vector.
pub struct Row {
    columns: Arc<Vec<wit::mysql::Column>>,
    result: Vec<DbValue>,
}

impl Row {
    /// Get a value by its column name. The value is converted to the target type as per the
    /// conversion table shown in the module documentation.
    ///
    /// This function returns None for both no such column _and_ failed conversion. You should use
    /// it only if you do not need to address errors (that is, if you know that conversion should
    /// never fail). If your code does not know the type in advance, use the raw [QueryResult::rows()] function
    /// instead of the [`QueryResult::next()`] or [`QueryResult::collect()`] wrappers to access
    /// the underlying [DbValue] enum: this will allow you to
    /// determine the type and process it accordingly.
    ///
    /// Additionally, this function performs a name lookup each time it is called. If you are iterating
    /// over a large number of rows, it's more efficient to use column indexes, either calculated or
    /// statically known from the column order in the SQL.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use spin_sdk::mysql::{Connection, DbValue};
    ///
    /// # async fn run() -> anyhow::Result<()> {
    /// # let user_id = 0;
    /// let db = Connection::open("mysql://root:my_password@localhost/mydb").await?;
    /// let mut query_result = db.query(
    ///     "SELECT * FROM users WHERE id = ?",
    ///     &[user_id.into()]
    /// ).await?;
    /// let user_row = query_result.next().await.unwrap();
    ///
    /// let name = user_row.get::<String>("name").unwrap();
    /// let age = user_row.get::<i16>("age").unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<T: Decode>(&self, column: &str) -> Option<T> {
        let i = self.columns.iter().position(|c| c.name == column)?;
        let db_value = self.result.get(i)?;
        Decode::decode(db_value).ok()
    }
}

impl std::ops::Index<usize> for Row {
    type Output = DbValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.result[index]
    }
}

/// A MySQL error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to deserialize [`DbValue`]
    #[error("error value decoding: {0}")]
    Decode(String),
    /// MySQL query failed with an error
    #[error(transparent)]
    MysqlError(#[from] MysqlError),
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
            DbValue::Int8(0) => Ok(false),
            DbValue::Int8(1) => Ok(true),
            _ => Err(Error::Decode(format_decode_err(
                "TINYINT(1), BOOLEAN",
                value,
            ))),
        }
    }
}

impl Decode for i8 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Int8(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("TINYINT", value))),
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

impl Decode for u8 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Uint8(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("UNSIGNED TINYINT", value))),
        }
    }
}

impl Decode for u16 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Uint16(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("UNSIGNED SMALLINT", value))),
        }
    }
}

impl Decode for u32 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Uint32(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err(
                "UNISIGNED MEDIUMINT, UNSIGNED INT",
                value,
            ))),
        }
    }
}

impl Decode for u64 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Uint64(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("UNSIGNED BIGINT", value))),
        }
    }
}

impl Decode for f32 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Floating32(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("FLOAT", value))),
        }
    }
}

impl Decode for f64 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Floating64(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("DOUBLE", value))),
        }
    }
}

impl Decode for Vec<u8> {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Binary(n) => Ok(n.to_owned()),
            _ => Err(Error::Decode(format_decode_err("BINARY, VARBINARY", value))),
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

fn format_decode_err(types: &str, value: &DbValue) -> String {
    format!("Expected {} from the DB but got {:?}", types, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean() {
        assert!(bool::decode(&DbValue::Int8(1)).unwrap());
        assert!(bool::decode(&DbValue::Int8(3)).is_err());
        assert!(bool::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<bool>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn int8() {
        assert_eq!(i8::decode(&DbValue::Int8(0)).unwrap(), 0);
        assert!(i8::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<i8>::decode(&DbValue::DbNull).unwrap().is_none());
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
    fn uint8() {
        assert_eq!(u8::decode(&DbValue::Uint8(0)).unwrap(), 0);
        assert!(u8::decode(&DbValue::Uint32(0)).is_err());
        assert!(Option::<u16>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn uint16() {
        assert_eq!(u16::decode(&DbValue::Uint16(0)).unwrap(), 0);
        assert!(u16::decode(&DbValue::Uint32(0)).is_err());
        assert!(Option::<u16>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn uint32() {
        assert_eq!(u32::decode(&DbValue::Uint32(0)).unwrap(), 0);
        assert!(u32::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<u32>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn uint64() {
        assert_eq!(u64::decode(&DbValue::Uint64(0)).unwrap(), 0);
        assert!(u64::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<u64>::decode(&DbValue::DbNull).unwrap().is_none());
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
        assert!(
            Option::<String>::decode(&DbValue::DbNull)
                .unwrap()
                .is_none()
        );
    }
}

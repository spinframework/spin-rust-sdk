#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "spin-sdk-sqlite",
        path: "wit",
        generate_all,
    });

    pub use spin::sqlite::sqlite;
}

#[doc(inline)]
pub use wit::sqlite::{Error, Value};

/// An open connection to a SQLite database.
///
/// [Connection::execute()] returns a tuple of `(columns, rows_stream, finish_future)`
/// where rows are consumed from a stream and the finish future is awaited to check
/// for errors.
///
/// # Examples
///
/// Open the default database, query rows, and iterate over the stream.
///
/// ```no_run
/// # async fn run() -> anyhow::Result<()> {
/// use spin_sdk::sqlite::{Connection, Value};
///
/// let min_age = 0;
/// let db = Connection::open_default().await?;
///
/// let mut query_result = db.execute(
///     "SELECT * FROM users WHERE age >= ?",
///     [Value::Integer(min_age)],
/// ).await?;
///
/// let name_idx = query_result.columns().iter().position(|c| c == "name").unwrap();
///
/// while let Some(row) = query_result.next().await {
///     let name: &str = row.get(name_idx).unwrap();
///     println!("Found user {name}");
/// }
///
/// query_result.result().await?;
/// # Ok(())
/// # }
/// ```
///
/// Perform an aggregate (scalar) operation over a named database.
///
/// ```no_run
/// # async fn run() -> anyhow::Result<()> {
/// use spin_sdk::sqlite::Connection;
///
/// let db = Connection::open("customer-data").await?;
/// let mut query_result = db.execute("SELECT COUNT(*) FROM users", []).await?;
///
/// if let Some(row) = query_result.next().await {
///     let count: i64 = row.get(0).unwrap();
///     println!("Total users: {count}");
/// }
///
/// query_result.result().await?;
/// # Ok(())
/// # }
/// ```
///
/// Delete rows from a database. The row stream will be empty, but the finish
/// future must still be awaited.
///
/// ```no_run
/// # async fn run() -> anyhow::Result<()> {
/// use spin_sdk::sqlite::{Connection, Value};
///
/// let min_age = 18;
/// let db = Connection::open("customer-data").await?;
/// let query_result = db.execute(
///     "DELETE FROM users WHERE age < ?",
///     [Value::Integer(min_age)],
/// ).await?;
///
/// query_result.result().await?;
/// # Ok(())
/// # }
/// ```
pub struct Connection(wit::sqlite::Connection);

impl Connection {
    /// Open a connection to the default database
    pub async fn open_default() -> Result<Self, Error> {
        Self::open("default").await
    }

    /// Open a connection to a named database instance.
    ///
    /// If `database` is "default", the default instance is opened.
    ///
    /// `error::no-such-database` will be raised if the `name` is not recognized.
    pub async fn open(database: impl AsRef<str>) -> Result<Self, Error> {
        wit::sqlite::Connection::open_async(database.as_ref().to_string())
            .await
            .map(Connection)
    }

    /// Execute a statement returning back data if there is any
    pub async fn execute(
        &self,
        statement: impl AsRef<str>,
        parameters: impl IntoIterator<Item = Value>,
    ) -> Result<QueryResult, Error> {
        let (columns, rows, result) = self
            .0
            .execute_async(
                statement.as_ref().to_string(),
                parameters.into_iter().collect(),
            )
            .await?;
        Ok(QueryResult {
            columns,
            rows,
            result,
        })
    }

    /// The SQLite rowid of the most recent successful INSERT on the connection, or 0 if
    /// there has not yet been an INSERT on the connection.
    pub async fn last_insert_rowid(&self) -> i64 {
        self.0.last_insert_rowid_async().await
    }

    /// The number of rows modified, inserted or deleted by the most recently completed
    /// INSERT, UPDATE or DELETE statement on the connection.
    pub async fn changes(&self) -> u64 {
        self.0.changes_async().await
    }
}

/// The result of a [`Connection::execute`] operation.
pub struct QueryResult {
    columns: Vec<String>,
    rows: wit_bindgen::StreamReader<RowResult>,
    result: wit_bindgen::FutureReader<Result<(), Error>>,
}

impl QueryResult {
    /// The columns in the query result.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Gets the next row in the result set.
    ///
    /// If this is `None`, there are no more rows available. You _must_
    /// await [`QueryResult::result()`] to determine if all rows
    /// were read successfully.
    pub async fn next(&mut self) -> Option<RowResult> {
        self.rows.next().await
    }

    /// Whether the query completed successfully or with an error.
    pub async fn result(self) -> Result<(), Error> {
        self.result.await
    }

    /// Collect all rows in the result set.
    ///
    /// This is provided for when the result set is small enough to fit in
    /// memory and you do not require streaming behaviour.
    pub async fn collect(self) -> Result<Vec<RowResult>, Error> {
        let rows = self.rows.collect().await;
        self.result.await?;
        Ok(rows)
    }

    /// Extracts the underlying Wasm Component Model results of the query.
    #[allow(clippy::type_complexity, reason = "that's what the inner bits are")]
    pub fn into_inner(
        self,
    ) -> (
        Vec<String>,
        wit_bindgen::StreamReader<RowResult>,
        wit_bindgen::FutureReader<Result<(), Error>>,
    ) {
        (self.columns, self.rows, self.result)
    }
}

/// A single row from a SQLite query result.
///
/// `RowResult` provides index-based access to column values via [`RowResult::get()`].
///
/// # Examples
///
/// Consume rows from the async streaming API:
///
/// ```no_run
/// # async fn run() -> anyhow::Result<()> {
/// use spin_sdk::sqlite::{Connection, Value};
///
/// let db = Connection::open_default().await?;
/// let mut query_result = db.execute(
///     "SELECT name, age FROM users WHERE age >= ?",
///     [Value::Integer(0)],
/// ).await?;
///
/// let name_idx = query_result.columns().iter().position(|c| c == "name").unwrap();
///
/// while let Some(row) = query_result.next().await {
///     let name: &str = row.get(name_idx).unwrap();
///     println!("Found user {name}");
/// }
///
/// query_result.result().await?;
/// # Ok(())
/// # }
/// ```
#[doc(inline)]
pub use wit::sqlite::RowResult;

impl RowResult {
    /// Get a value by its column name. The value is converted to the target type.
    ///
    /// * SQLite integers are convertible to Rust integer types (i8, u8, i16, etc. including usize and isize) and bool.
    /// * SQLite strings are convertible to Rust &str or &[u8] (encoded as UTF-8).
    /// * SQLite reals are convertible to Rust f64.
    /// * SQLite blobs are convertible to Rust &[u8] or &str (interpreted as UTF-8).
    ///
    /// To look up by name, you can use `QueryResult::rows()` or obtain the invoice from `QueryResult::columns`.
    /// If you do not know the type of a value, access the underlying [Value] enum directly
    /// via the [RowResult::values] field
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run() -> anyhow::Result<()> {
    /// use spin_sdk::sqlite::{Connection, Value};
    ///
    /// let db = Connection::open_default().await?;
    /// let mut query_result = db.execute(
    ///     "SELECT name, age FROM users WHERE id = ?",
    ///     [Value::Integer(0)],
    /// ).await?;
    ///
    /// if let Some(row) = query_result.next().await {
    ///     let name: &str = row.get(0).unwrap();
    ///     let age: u16 = row.get(1).unwrap();
    ///     println!("{name} is {age} years old");
    /// }
    ///
    /// query_result.result().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<'a, T: TryFrom<&'a Value>>(&'a self, index: usize) -> Option<T> {
        self.values.get(index).and_then(|c| c.try_into().ok())
    }
}

impl<'a> TryFrom<&'a Value> for bool {
    type Error = ();

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(i) => Ok(*i != 0),
            _ => Err(()),
        }
    }
}

macro_rules! int_conversions {
    ($($t:ty),*) => {
        $(impl<'a> TryFrom<&'a Value> for $t {
            type Error = ();

            fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
                match value {
                    Value::Integer(i) => (*i).try_into().map_err(|_| ()),
                    _ => Err(()),
                }
            }
        })*
    };
}

int_conversions!(u8, u16, u32, u64, i8, i16, i32, i64, usize, isize);

impl<'a> TryFrom<&'a Value> for f64 {
    type Error = ();

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Real(f) => Ok(*f),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = ();

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Text(s) => Ok(s.as_str()),
            Value::Blob(b) => std::str::from_utf8(b).map_err(|_| ()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a [u8] {
    type Error = ();

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Blob(b) => Ok(b.as_slice()),
            Value::Text(s) => Ok(s.as_bytes()),
            _ => Err(()),
        }
    }
}

use super::wit::v2::sqlite;

#[doc(inline)]
pub use sqlite::{Error, Value};

/// An open connection to a SQLite database.
///
/// # Examples
///
/// Load a set of rows from the default SQLite database, and iterate over them.
///
/// ```no_run
/// use spin_sdk::sqlite::{Connection, Value};
///
/// # fn main() -> anyhow::Result<()> {
/// # let min_age = 0;
/// let db = Connection::open_default()?;
///
/// let query_result = db.execute(
///     "SELECT * FROM users WHERE age >= ?",
///     &[Value::Integer(min_age)]
/// )?;
///
/// let name_index = query_result.columns.iter().position(|c| c == "name").unwrap();
///
/// for row in &query_result.rows {
///     let name: &str = row.get(name_index).unwrap();
///     println!("Found user {name}");
/// }
/// # Ok(())
/// # }
/// ```
///
/// Use the [QueryResult::rows()] wrapper to access a column by name. This is simpler and
/// more readable but incurs a lookup on each access, so is not recommended when
/// iterating a data set.
///
/// ```no_run
/// # use spin_sdk::sqlite::{Connection, Value};
/// # fn main() -> anyhow::Result<()> {
/// # let user_id = 0;
/// let db = Connection::open_default()?;
/// let query_result = db.execute(
///     "SELECT * FROM users WHERE id = ?",
///     &[Value::Integer(user_id)]
/// )?;
/// let name = query_result.rows().next().and_then(|r| r.get::<&str>("name")).unwrap();
/// # Ok(())
/// # }
/// ```
///
/// Perform an aggregate (scalar) operation over a named SQLite database. The result
/// set contains a single column, with a single row.
///
/// ```no_run
/// use spin_sdk::sqlite::Connection;
///
/// # fn main() -> anyhow::Result<()> {
/// # let user_id = 0;
/// let db = Connection::open("customer-data")?;
/// let query_result = db.execute("SELECT COUNT(*) FROM users", &[])?;
/// let count = query_result.rows.first().and_then(|r| r.get::<usize>(0)).unwrap();
/// # Ok(())
/// # }
/// ```
///
/// Delete rows from a SQLite database. The usual [Connection::execute()] syntax
/// is used but the query result is always empty.
///
/// ```no_run
/// use spin_sdk::sqlite::{Connection, Value};
///
/// # fn main() -> anyhow::Result<()> {
/// # let min_age = 18;
/// let db = Connection::open("customer-data")?;
/// db.execute("DELETE FROM users WHERE age < ?", &[Value::Integer(min_age)])?;
/// # Ok(())
/// # }
/// ```
#[doc(inline)]
pub use sqlite::Connection;

/// The result of a SQLite query issued with [Connection::execute()].
///
/// # Examples
///
/// Load a set of rows from the default SQLite database, and iterate over them.
///
/// ```no_run
/// use spin_sdk::sqlite::{Connection, Value};
///
/// # fn main() -> anyhow::Result<()> {
/// # let min_age = 0;
/// let db = Connection::open_default()?;
///
/// let query_result = db.execute(
///     "SELECT * FROM users WHERE age >= ?",
///     &[Value::Integer(min_age)]
/// )?;
///
/// let name_index = query_result.columns.iter().position(|c| c == "name").unwrap();
///
/// for row in &query_result.rows {
///     let name: &str = row.get(name_index).unwrap();
///     println!("Found user {name}");
/// }
/// # Ok(())
/// # }
/// ```
///
/// Use the [QueryResult::rows()] wrapper to access a column by name. This is simpler and
/// more readable but incurs a lookup on each access, so is not recommended when
/// iterating a data set.
///
/// ```no_run
/// # use spin_sdk::sqlite::{Connection, Value};
/// # fn main() -> anyhow::Result<()> {
/// # let user_id = 0;
/// let db = Connection::open_default()?;
/// let query_result = db.execute(
///     "SELECT * FROM users WHERE id = ?",
///     &[Value::Integer(user_id)]
/// )?;
/// let name = query_result.rows().next().and_then(|r| r.get::<&str>("name")).unwrap();
/// # Ok(())
/// # }
/// ```
///
/// Perform an aggregate (scalar) operation over a named SQLite database. The result
/// set contains a single column, with a single row.
///
/// ```no_run
/// use spin_sdk::sqlite::Connection;
///
/// # fn main() -> anyhow::Result<()> {
/// # let user_id = 0;
/// let db = Connection::open("customer-data")?;
/// let query_result = db.execute("SELECT COUNT(*) FROM users", &[])?;
/// let count = query_result.rows.first().and_then(|r| r.get::<usize>(0)).unwrap();
/// # Ok(())
/// # }
/// ```
#[doc(inline)]
pub use sqlite::QueryResult;

/// A database row result.
///
/// There are two representations of a SQLite row in the SDK. This type is obtained from
/// the [field@QueryResult::rows] field, and provides index-based lookup or low-level access
/// to row values via a vector. The [Row] type is useful for
/// addressing elements by column name, and is obtained from the [QueryResult::rows()] function.
///
/// # Examples
///
/// Load a set of rows from the default SQLite database, and iterate over them selecting one
/// field from each. The example caches the index of the desired field to avoid repeated lookup,
/// making this more efficient than the [Row]-based equivalent at the expense of
/// extra code and inferior readability.
///
/// ```no_run
/// use spin_sdk::sqlite::{Connection, Value};
///
/// # fn main() -> anyhow::Result<()> {
/// # let min_age = 0;
/// let db = Connection::open_default()?;
///
/// let query_result = db.execute(
///     "SELECT * FROM users WHERE age >= ?",
///     &[Value::Integer(min_age)]
/// )?;
///
/// let name_index = query_result.columns.iter().position(|c| c == "name").unwrap();
///
/// for row in &query_result.rows {
///     let name: &str = row.get(name_index).unwrap();
///     println!("Found user {name}");
/// }
/// # Ok(())
/// # }
/// ```
#[doc(inline)]
pub use sqlite::RowResult;

impl sqlite::Connection {
    /// Open a connection to the default database
    pub fn open_default() -> Result<Self, Error> {
        Self::open("default")
    }
}

impl sqlite::QueryResult {
    /// Get all the rows for this query result
    pub fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        self.rows.iter().map(|r| Row {
            columns: self.columns.as_slice(),
            result: r,
        })
    }
}

/// A database row result.
///
/// There are two representations of a SQLite row in the SDK.  This type is useful for
/// addressing elements by column name, and is obtained from the [QueryResult::rows()] function.
/// The [RowResult] type is obtained from the [field@QueryResult::rows] field, and provides
/// index-based lookup or low-level access to row values via a vector.
pub struct Row<'a> {
    columns: &'a [String],
    result: &'a sqlite::RowResult,
}

impl<'a> Row<'a> {
    /// Get a value by its column name. The value is converted to the target type.
    ///
    /// * SQLite integers are convertible to Rust integer types (i8, u8, i16, etc. including usize and isize) and bool.
    /// * SQLite strings are convertible to Rust &str or &[u8] (encoded as UTF-8).
    /// * SQLite reals are convertible to Rust f64.
    /// * SQLite blobs are convertible to Rust &[u8] or &str (interpreted as UTF-8).
    ///
    /// If your code does not know the type in advance, use [RowResult] instead of `Row` to
    /// access the underlying [Value] enum.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use spin_sdk::sqlite::{Connection, Value};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// # let user_id = 0;
    /// let db = Connection::open_default()?;
    /// let query_result = db.execute(
    ///     "SELECT * FROM users WHERE id = ?",
    ///     &[Value::Integer(user_id)]
    /// )?;
    /// let user_row = query_result.rows().next().unwrap();
    ///
    /// let name = user_row.get::<&str>("name").unwrap();
    /// let age = user_row.get::<u16>("age").unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<T: TryFrom<&'a Value>>(&self, column: &str) -> Option<T> {
        let i = self.columns.iter().position(|c| c == column)?;
        self.result.get(i)
    }
}

impl sqlite::RowResult {
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
    /// use spin_sdk::sqlite::{Connection, Value};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// # let user_id = 0;
    /// let db = Connection::open_default()?;
    /// let query_result = db.execute(
    ///     "SELECT name, age FROM users WHERE id = ?",
    ///     &[Value::Integer(user_id)]
    /// )?;
    /// let user_row = query_result.rows.first().unwrap();
    ///
    /// let name = user_row.get::<&str>(0).unwrap();
    /// let age = user_row.get::<u16>(1).unwrap();
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

use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::{Pool, Sqlite};
use std::fmt::Display;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::tables::{ColumnId, ColumnSchema, ColumnType, TableId};

const USER_DATA_DIR: &str = "user_data";
const DB_FILENAME: &str = "fitness_tracker.db";

// Storage for a user table schema
pub struct TableSchemaRow {
    pub table_id: TableId,
    pub name: String,
}

// ColumnSchemaRow: Encodes the column-belongs-to-table relation.
// ColumnSchemaRow would be identical to the ColumnSchema domain type;
// reuse the struct in the database layer.

/// Creates the `table_schemas` and `column_schemas` tables if they don't
/// already exist. `column_schemas` rows reference their owning table with
/// `ON DELETE CASCADE`, so deleting a table schema also deletes its columns.
async fn create_schema_tables(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS table_schemas (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS column_schemas (
            id TEXT PRIMARY KEY NOT NULL,
            table_id TEXT NOT NULL REFERENCES table_schemas(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            ty TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Creates a new table schema named `name` with a default "Name" column,
/// persists both to the database, and returns the resulting table schema
/// row.
pub async fn create_table(
    pool: &Pool<Sqlite>,
    name: impl Into<String>,
) -> anyhow::Result<TableSchemaRow> {
    let table_id = TableId::new();
    let name = name.into();

    sqlx::query("INSERT INTO table_schemas (id, name) VALUES (?, ?)")
        .bind(table_id.0.to_string())
        .bind(&name)
        .execute(pool)
        .await?;

    let name_column = ColumnSchema {
        id: ColumnId::new(),
        table_id: table_id.clone(),
        name: "Name".into(),
        ty: ColumnType::String,
    };

    sqlx::query("INSERT INTO column_schemas (id, table_id, name, ty) VALUES (?, ?, ?, ?)")
        .bind(name_column.id.0.to_string())
        .bind(name_column.table_id.0.to_string())
        .bind(&name_column.name)
        .bind(&name_column.ty.to_string())
        .execute(pool)
        .await?;

    Ok(TableSchemaRow { table_id, name })
}

/// Deletes the table schema for `id`. Cascades to delete its column schemas
/// via the `ON DELETE CASCADE` foreign key. Returns an error if no table
/// schema exists for `id`.
pub async fn delete_table(pool: &Pool<Sqlite>, id: &TableId) -> anyhow::Result<()> {
    let result = sqlx::query("DELETE FROM table_schemas WHERE id = ?")
        .bind(id.0.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("no table schema found for id {}", id.0);
    }

    Ok(())
}

/// Creates a column associated with a table with the given name and type.
pub async fn create_column(
    pool: &Pool<Sqlite>,
    table_id: &TableId,
    name: impl Into<String> + Display,
    ty: ColumnType,
) -> anyhow::Result<ColumnSchema> {
    let column_id = ColumnId::new();
    let name = name.into();

    sqlx::query("INSERT INTO column_schemas (id, table_id, name, ty) VALUES (?, ?, ?, ?)")
        .bind(column_id.0.to_string())
        .bind(table_id.0.to_string())
        .bind(&name)
        .bind(ty.to_string())
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "creating new column {} in table with id {}",
                name, table_id.0
            )
        })?;

    Ok(ColumnSchema {
        id: column_id,
        table_id: table_id.clone(),
        name,
        ty,
    })
}

/// Verifies (or creates, with user consent) `base_dir`. Returns whether the
/// directory was freshly created by this call.
fn verify_user_data_dir(
    base_dir: &Path,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> io::Result<bool> {
    if base_dir.is_dir() {
        writeln!(writer, "Found local data.")?;
        return Ok(false);
    }

    writeln!(writer, "No user data found.")?;
    write!(writer, "Create user_data directory? (y/n): ")?;
    writer.flush()?;

    let mut input = String::new();
    reader.read_line(&mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        fs::create_dir(base_dir)?;
        writeln!(writer, "Created user_data directory.")?;
        Ok(true)
    } else {
        writeln!(writer, "Skipped creating user_data directory.")?;
        Ok(false)
    }
}

/// Opens a handle to the SQLite database under `base_dir`. If `allow_create`
/// is false and the database file doesn't already exist, returns an error
/// instructing the user to delete or move `base_dir` before retrying. This
/// leaves ownership of the expected base dir with the user.
async fn open_db(base_dir: &Path, allow_create: bool) -> anyhow::Result<Pool<Sqlite>> {
    let db_path = base_dir.join(DB_FILENAME);

    if !allow_create && !db_path.is_file() {
        anyhow::bail!(
            "No SQLite database found at {}. Delete or move the {} directory, \
             then retry to create a fresh data directory.",
            db_path.display(),
            USER_DATA_DIR
        );
    }

    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(allow_create)
        .pragma("foreign_keys", "ON");

    let pool = SqlitePool::connect_with(options).await?;

    create_schema_tables(&pool).await?;

    Ok(pool)
}

/// Verifies (or creates) the user_data directory and opens a handle to the
/// SQLite database within it, creating the database file if it doesn't
/// already exist.
pub async fn init_user_data() -> anyhow::Result<Pool<Sqlite>> {
    let base_dir = Path::new(USER_DATA_DIR);

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();
    let dir_was_created = verify_user_data_dir(base_dir, &mut reader, &mut stdout)?;

    open_db(base_dir, dir_was_created).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn stdin(input: &str) -> Cursor<Vec<u8>> {
        Cursor::new(input.as_bytes().to_vec())
    }

    // A missing user_data file prompts user for creation.
    #[test]
    fn missing_dir_prompts_for_creation() {
        let dir = tempfile::tempdir().unwrap();
        let base_dir = dir.path().join("user_data");

        let mut input = stdin("n\n");
        let mut output = Vec::new();
        verify_user_data_dir(&base_dir, &mut input, &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("No user data found."));
        assert!(output.contains("Create user_data directory? (y/n): "));
    }

    // Accepting file creation produces file and database.
    #[tokio::test]
    async fn accepting_creation_produces_dir_and_database() {
        let dir = tempfile::tempdir().unwrap();
        let base_dir = dir.path().join("user_data");

        let mut input = stdin("y\n");
        let mut output = Vec::new();
        let dir_was_created = verify_user_data_dir(&base_dir, &mut input, &mut output).unwrap();

        assert!(dir_was_created);
        assert!(base_dir.is_dir());

        let pool = open_db(&base_dir, dir_was_created).await.unwrap();
        assert!(base_dir.join(DB_FILENAME).is_file());
        pool.close().await;
    }

    // Denying file creation exists gracefully.
    #[test]
    fn denying_creation_exits_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let base_dir = dir.path().join("user_data");

        let mut input = stdin("n\n");
        let mut output = Vec::new();
        let dir_was_created = verify_user_data_dir(&base_dir, &mut input, &mut output).unwrap();

        assert!(!dir_was_created);
        assert!(!base_dir.is_dir());
    }

    // Attaching to valid database returns appropriate SQLx handle.
    #[tokio::test]
    async fn attaching_to_valid_database_returns_handle() {
        let dir = tempfile::tempdir().unwrap();
        let base_dir = dir.path().join("user_data");
        fs::create_dir(&base_dir).unwrap();

        open_db(&base_dir, true).await.unwrap().close().await;

        let pool = open_db(&base_dir, false).await;
        assert!(pool.is_ok());
        pool.unwrap().close().await;
    }

    // A missing SQLite database under user_data/ returns an error and prompts
    // user to either delete or move user_data before retrying.
    #[tokio::test]
    async fn missing_database_under_existing_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let base_dir = dir.path().join("user_data");
        fs::create_dir(&base_dir).unwrap();

        let result = open_db(&base_dir, false).await;

        let err = result.expect_err("expected missing database to error");
        let message = err.to_string();
        assert!(message.contains("Delete or move"));
    }

    // Creating a table adds a default "Name" column; deleting it cascades to
    // remove that column too; deleting the (now-nonexistent) table again
    // errors.
    #[tokio::test]
    async fn table_create_then_delete_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let base_dir = dir.path().join("user_data");
        fs::create_dir(&base_dir).unwrap();
        let pool = open_db(&base_dir, true).await.unwrap();

        let table = create_table(&pool, "workouts").await.unwrap();
        assert_eq!(table.name, "workouts");

        let (name_column_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM column_schemas WHERE table_id = ? AND name = ?")
                .bind(table.table_id.0.to_string())
                .bind("Name")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(name_column_count, 1);

        delete_table(&pool, &table.table_id).await.unwrap();

        let (table_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM table_schemas WHERE id = ?")
                .bind(table.table_id.0.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(table_count, 0);

        let (column_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM column_schemas WHERE table_id = ?")
                .bind(table.table_id.0.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(column_count, 0);

        let result = delete_table(&pool, &table.table_id).await;
        assert!(result.is_err());

        pool.close().await;
    }

    // Creating a column on a preexisting table persists it alongside the
    // default "Name" column.
    #[tokio::test]
    async fn create_column_persists_column_on_existing_table() {
        let dir = tempfile::tempdir().unwrap();
        let base_dir = dir.path().join("user_data");
        fs::create_dir(&base_dir).unwrap();
        let pool = open_db(&base_dir, true).await.unwrap();

        let table = create_table(&pool, "workouts").await.unwrap();

        let column = create_column(&pool, &table.table_id, "reps", ColumnType::Integer)
            .await
            .unwrap();

        assert_eq!(column.table_id, table.table_id);
        assert_eq!(column.name, "reps");
        assert!(matches!(column.ty, ColumnType::Integer));

        let (name, ty): (String, String) =
            sqlx::query_as("SELECT name, ty FROM column_schemas WHERE id = ?")
                .bind(column.id.0.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(name, "reps");
        assert_eq!(ty, ColumnType::Integer.to_string());

        let (column_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM column_schemas WHERE table_id = ?")
                .bind(table.table_id.0.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(column_count, 2);

        pool.close().await;
    }
}

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::{Pool, Sqlite};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

const USER_DATA_DIR: &str = "user_data";
const DB_FILENAME: &str = "fitness_tracker.db";

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
        .create_if_missing(allow_create);

    let pool = SqlitePool::connect_with(options).await?;

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
}

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
    pool: sqlx::SqlitePool,
}

pub const MAX_SQLITE_RETRY_ATTEMPTS: u32 = 3;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("directory does not exist: {0}.")]
    DirectoryMissing(String),

    #[error("database file does not exist: {0}.")]
    FileMissing(String),

    #[error("file already exists: {0}")]
    FileExists(String),

    #[error("retry limit exceeded")]
    RetryLimitExceeded(String),

    #[error("database files in {0} are corrupted.")]
    CorruptedFiles(String),

    #[error("failed to connect to database")]
    ConnectionFailed(#[from] std::io::Error),
}

impl Database {
    async fn verify_application_tables(pool: &Pool<Sqlite>) -> bool {
        let count: Result<(i64,), _> = sqlx::query_as(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_one(pool)
        .await;

        matches!(count, Ok((0,)))
    }

    async fn create_database_file(path: &Path) -> Result<(), DatabaseError> {
        if path.exists() {
            return Err(DatabaseError::FileExists(path.display().to_string()));
        }

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);

        sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(|e| {
                DatabaseError::ConnectionFailed(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?
            .close()
            .await;

        Ok(())
    }

    async fn connect(path: &Path) -> Result<Database, DatabaseError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(DatabaseError::DirectoryMissing(
                    parent.display().to_string(),
                ));
            }
        }

        if !path.exists() {
            return Err(DatabaseError::FileMissing(path.display().to_string()));
        }

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false);

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(|e| {
                DatabaseError::ConnectionFailed(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        if !Database::verify_application_tables(&pool).await {
            return Err(DatabaseError::CorruptedFiles(path.display().to_string()));
        }

        Ok(Database {
            path: path.into(),
            pool,
        })
    }

    pub async fn connect_with_retry(path: &Path) -> Result<Database, DatabaseError> {
        let mut last_err = None;

        for _ in 0..MAX_SQLITE_RETRY_ATTEMPTS {
            match Database::connect(path).await {
                Ok(db) => return Ok(db),
                Err(e @ DatabaseError::FileMissing(_)) => {
                    Database::create_database_file(path).await?;
                    last_err = Some(e);
                }
                Err(
                    e @ (DatabaseError::DirectoryMissing(_)
                    | DatabaseError::FileExists(_)
                    | DatabaseError::CorruptedFiles(_)
                    | DatabaseError::ConnectionFailed(_)
                    | DatabaseError::RetryLimitExceeded(_)),
                ) => return Err(e),
            }
        }

        Err(last_err
            .unwrap_or_else(|| DatabaseError::RetryLimitExceeded(path.display().to_string())))
    }

    pub async fn try_new(path: &Path) -> Result<Database, DatabaseError> {
        Database::connect_with_retry(path).await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_missing_file() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("db.sqlite");
        // Don't create file.

        let db = Database::try_new(&db_path)
            .await
            .expect("try_new should succeed when the file is missing");

        assert!(db_path.exists());
        assert_eq!(db.path, db_path);
        assert!(Database::verify_application_tables(&db.pool).await);

        let reconnected = Database::try_new(&db_path)
            .await
            .expect("reconnecting to an existing database should succeed");

        assert_eq!(reconnected.path, db_path);
        assert!(Database::verify_application_tables(&reconnected.pool).await);
    }
}

mod repository;
mod tables;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let _db = match repository::init_user_data().await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };

    let col_schema = tables::ColumnSchema::new("col1", tables::ColumnType::String);
    let tab_schema = tables::TableSchema::new("tab1", vec![col_schema]);

    // TODO: enter primary application

    ExitCode::SUCCESS
}

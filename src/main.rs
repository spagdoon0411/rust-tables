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

    let table = match repository::create_table(&_db, "tab1").await {
        Ok(table) => table,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };

    let new_columns = [
        ("col2", tables::ColumnType::Integer),
        ("col3", tables::ColumnType::Boolean),
        ("col4", tables::ColumnType::Real),
    ];

    let mut columns = Vec::new();
    for (name, ty) in new_columns {
        match repository::create_column(&_db, &table.table_id, name, ty).await {
            Ok(column) => columns.push(column),
            Err(err) => {
                eprintln!("error: {err}");
                return ExitCode::FAILURE;
            }
        }
    }

    let tab_schema = tables::TableSchema {
        id: table.table_id.clone(),
        name: table.name.clone(),
        columns: columns.clone(),
    };

    println!("{tab_schema:?}");

    if let Err(err) = repository::delete_table(&_db, &table.table_id).await {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }

    // TODO: remove table creation example and launch TUI here

    ExitCode::SUCCESS
}

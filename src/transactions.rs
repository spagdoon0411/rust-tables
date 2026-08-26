use tokio::sync::mpsc;

use sqlx::{Pool, Sqlite};

use crate::{
    repository::{self, TableSchemaRow},
    tables::TableId,
};

async fn create_table(
    pool: Pool<Sqlite>,
    input: CreateTableInput,
) -> anyhow::Result<CreateTableOutput> {
    let table = repository::create_table(pool, input.name).await?;
    Ok(CreateTableOutput { table })
}

async fn delete_table(
    pool: Pool<Sqlite>,
    input: DeleteTableInput,
) -> anyhow::Result<DeleteTableOutput> {
    repository::delete_table(pool, &input.table_id).await?;
    Ok(DeleteTableOutput {})
}

async fn retrieve_tables(pool: Pool<Sqlite>) -> anyhow::Result<RetrieveTablesOutput> {
    let _ = pool;
    // Repository has no listing query yet.
    Ok(RetrieveTablesOutput { tables: vec![] })
}

pub struct CreateTableInput {
    name: String,
}
pub struct CreateTableOutput {
    table: TableSchemaRow,
}

pub struct DeleteTableInput {
    table_id: TableId,
}
pub struct DeleteTableOutput {}

pub struct RetrieveTablesOutput {
    tables: Vec<TableSchemaRow>,
}

pub enum AppOperationRequest {
    CreateTable(CreateTableInput),
    DeleteTable(DeleteTableInput),
    RetrieveTables,
}

pub enum AppOperationResult {
    CreateTable(anyhow::Result<CreateTableOutput>),
    DeleteTable(anyhow::Result<DeleteTableOutput>),
    RetrieveTables(anyhow::Result<RetrieveTablesOutput>),
}

/// Dispatches `request` to its operation-specific function, passing the
/// whole input struct rather than unpacked fields.
async fn execute_request(pool: Pool<Sqlite>, request: AppOperationRequest) -> AppOperationResult {
    match request {
        AppOperationRequest::CreateTable(input) => {
            AppOperationResult::CreateTable(create_table(pool, input).await)
        }
        AppOperationRequest::DeleteTable(input) => {
            AppOperationResult::DeleteTable(delete_table(pool, input).await)
        }
        AppOperationRequest::RetrieveTables => {
            AppOperationResult::RetrieveTables(retrieve_tables(pool).await)
        }
    }
}

/// Spawns `request`'s execution against `pool` and sends its result over
/// `tx` once complete, without blocking the caller.
fn launch(tx: mpsc::Sender<AppOperationResult>, pool: Pool<Sqlite>, request: AppOperationRequest) {
    tokio::spawn(async move {
        // TODO: may be incomplete on a UI shutdown (app exits; Tokio tasks are killed)
        let result = execute_request(pool, request).await;
        let _ = tx.send(result).await;
    });
}

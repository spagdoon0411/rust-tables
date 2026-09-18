use std::path::Path;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::database::Database;
use crate::dispatcher::Action;

#[derive(Debug, PartialEq, Clone)]
pub struct TableSchema {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub enum AsyncOperationRequest {
    ListTables,
}

#[derive(Debug, PartialEq)]
pub enum AsyncOperationResult {
    ListTables(Vec<TableSchema>),
}

pub struct SQLiteAPIClient {
    db: Database,
    pub request_tx: mpsc::Sender<AsyncOperationRequest>,
    request_rx: mpsc::Receiver<AsyncOperationRequest>,
    pub result_tx: mpsc::Sender<Action>,
}

impl SQLiteAPIClient {
    pub const REQUEST_CHANNEL_SIZE: usize = 256;

    // TODO: define APIClient error type
    pub async fn try_new(path: &Path, result_tx: mpsc::Sender<Action>) -> anyhow::Result<Self> {
        let db = Database::try_new(path).await?;
        let (request_tx, request_rx) = mpsc::channel(SQLiteAPIClient::REQUEST_CHANNEL_SIZE);

        Ok(Self {
            db,
            request_tx,
            request_rx,
            result_tx,
        })
    }

    pub async fn list_tables(db: Database) -> Vec<TableSchema> {
        db.retrieve_tables()
            .await
            .unwrap_or(vec![])
            .into_iter()
            .map(|row| TableSchema {
                id: row.id,
                name: row.name,
            })
            .collect()
    }

    async fn delegate_request(
        db: Database,
        request: AsyncOperationRequest,
    ) -> AsyncOperationResult {
        match request {
            AsyncOperationRequest::ListTables => {
                AsyncOperationResult::ListTables(Self::list_tables(db).await)
            }
        }
    }
}

#[async_trait]
pub trait AppAPIClient {
    fn handle_request(&self, request: AsyncOperationRequest);
    async fn next_request(&mut self) -> AsyncOperationRequest;
}

#[async_trait]
impl AppAPIClient for SQLiteAPIClient {
    fn handle_request(&self, request: AsyncOperationRequest) {
        let db_clone = self.db.clone();
        let tx_clone = self.result_tx.clone();

        tokio::spawn(async move {
            // TODO: log hangup
            let _ = tx_clone.send(Action::AsyncOperationRequest(request.clone()));

            let result = Action::AsyncOperationResult(
                Self::delegate_request(db_clone, request.clone()).await,
            );

            // TODO: log hangup
            let _ = tx_clone.send(result).await;
        });
    }

    async fn next_request(&mut self) -> AsyncOperationRequest {
        loop {
            match self.request_rx.recv().await {
                Some(request) => return request,
                None => {}
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_list_tables_request_flow() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("db.sqlite");

        let (result_tx, mut result_rx) = mpsc::channel(1);
        let mut client = SQLiteAPIClient::try_new(&db_path, result_tx)
            .await
            .expect("client should connect to the mocked database");

        // 1. A request_tx message: send the request in, the way a component would.
        client
            .request_tx
            .send(AsyncOperationRequest::ListTables)
            .await
            .expect("request_tx should accept a ListTables request");

        let request = client.next_request().await;
        client.handle_request(request);

        // 2. The result_tx message: the spawned request-handling task reports back.
        let result = result_rx
            .recv()
            .await
            .expect("result_tx should deliver the ListTables result");

        // 3. The returned value: an empty database has no tables to list.
        match result {
            Action::AsyncOperationResult(AsyncOperationResult::ListTables(tables)) => {
                assert_eq!(tables, vec![]);
            }
            _ => panic!("expected an AsyncOperationResult::ListTables action"),
        }
    }
}

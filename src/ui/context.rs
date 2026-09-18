use crate::{clients::AsyncOperationRequest, dispatcher::Action, store::SharedStore};

use std::sync::Arc;
use tokio::sync::mpsc;

pub struct SharedContext {
    pub store: SharedStore,
    pub request_tx: mpsc::Sender<AsyncOperationRequest>,
    pub action_tx: mpsc::Sender<Action>,
}

impl SharedContext {
    pub fn new(
        store: SharedStore,
        request_tx: mpsc::Sender<AsyncOperationRequest>,
        action_tx: mpsc::Sender<Action>,
    ) -> Arc<Self> {
        Arc::new(Self {
            store,
            request_tx,
            action_tx,
        })
    }
}

use tokio::sync::mpsc;

use crate::clients::{AsyncOperationRequest, AsyncOperationResult};

pub enum Action {
    AsyncOperationRequest(AsyncOperationRequest),
    AsyncOperationResult(AsyncOperationResult),
    SelectNextTable,
    SelectPreviousTable,
}

pub struct Dispatcher {
    pub action_tx: mpsc::Sender<Action>,
    action_rx: mpsc::Receiver<Action>,
}

impl Dispatcher {
    pub const ACTION_CHANNEL_SIZE: usize = 256;

    pub fn new() -> Self {
        let (action_tx, action_rx) = mpsc::channel(Dispatcher::ACTION_CHANNEL_SIZE);

        Self {
            action_tx,
            action_rx,
        }
    }

    pub async fn next_action(&mut self) -> Action {
        loop {
            if let Some(action) = self.action_rx.recv().await {
                return action;
            }
        }
    }
}

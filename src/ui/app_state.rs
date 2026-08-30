use super::Renderable;
use super::UserActionEvent;
use super::page_state::PageState;
use crate::transactions::{AppOperationRequest, AppOperationResult};
use crossterm::event::EventStream;
use ratatui::Frame;

pub struct AppState {
    pub page_state: PageState,
}

impl Renderable for AppState {
    // The app's transitions are driven entirely by its inner page state.
    type Next = AppState;

    fn draw(&mut self, frame: &mut Frame) {
        self.page_state.draw(frame);
    }

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent> {
        self.page_state.collect_action(event_stream).await
    }

    fn next_state_from_user_action(
        self,
        action: &UserActionEvent,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let (page_state, request) = self.page_state.next_state_from_user_action(action)?;
        Ok((AppState { page_state }, request))
    }

    fn next_state_from_async_message(
        self,
        msg: &AppOperationResult,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let (page_state, request) = self.page_state.next_state_from_async_message(msg)?;
        Ok((AppState { page_state }, request))
    }

    fn next_state_from_tick(self) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let (page_state, request) = self.page_state.next_state_from_tick()?;
        Ok((AppState { page_state }, request))
    }
}

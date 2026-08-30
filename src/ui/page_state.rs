pub use super::home_page::HomePage;
pub use super::table_page::TablePage;
pub use super::{AppEvent, Renderable, UserActionEvent};

use anyhow::Context;
use crossterm::event::EventStream;
use ratatui::{DefaultTerminal, Frame};
use std::mem::discriminant;

use crate::transactions::{AppOperationRequest, AppOperationResult};

// TODO: make AppState UI-agnostic?
pub enum PageState {
    HomePage(HomePage),
    TablePage(TablePage),
    Exited,
}

impl Renderable for PageState {
    type Next = PageState;

    fn draw(&mut self, frame: &mut Frame) {
        match self {
            PageState::HomePage(page) => page.draw(frame),
            PageState::TablePage(page) => page.draw(frame),
            PageState::Exited => (),
        }
    }

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent> {
        match self {
            PageState::HomePage(page) => page.collect_action(event_stream).await,
            PageState::TablePage(page) => page.collect_action(event_stream).await,
            PageState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }

    fn next_state_from_user_action(
        self,
        action: &UserActionEvent,
    ) -> anyhow::Result<(PageState, Option<AppOperationRequest>)> {
        match self {
            PageState::HomePage(page) => page.next_state_from_user_action(action),
            PageState::TablePage(page) => page.next_state_from_user_action(action),
            PageState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }

    fn next_state_from_async_message(
        self,
        msg: &AppOperationResult,
    ) -> anyhow::Result<(PageState, Option<AppOperationRequest>)> {
        match self {
            PageState::HomePage(page) => page.next_state_from_async_message(msg),
            PageState::TablePage(page) => page.next_state_from_async_message(msg),
            PageState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }

    fn next_state_from_tick(self) -> anyhow::Result<(PageState, Option<AppOperationRequest>)> {
        match self {
            PageState::HomePage(page) => page.next_state_from_tick(),
            PageState::TablePage(page) => page.next_state_from_tick(),
            PageState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }
}

impl PageState {
    pub fn transition_app_state(
        self,
        app_event: &AppEvent,
        terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<(PageState, Option<AppOperationRequest>)> {
        let current_kind = discriminant(&self);

        let (next_state, request) = self.next_state_from_event(app_event)?;

        // Clear terminal on page changes
        let next_kind = discriminant(&next_state);
        if next_kind != current_kind {
            terminal.clear().context("while clearing terminal")?;
        }

        Ok((next_state, request))
    }
}

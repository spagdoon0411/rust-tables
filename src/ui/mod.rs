mod home_page;
mod notifications;
mod table_page;

pub use home_page::HomePage;
pub use table_page::TablePage;

use notifications::NotifLevel;
use std::mem::discriminant;

use anyhow::Context;
use crossterm::event::EventStream;
use ratatui::{DefaultTerminal, Frame};

use crate::{
    tables::{ColumnType, TableSchema},
    transactions::{AppOperationRequest, AppOperationResult},
};

pub enum ScrollDirection {
    Left,
    Down,
    Up,
    Right,
}

/// Actions that can be collected by a UI.
pub enum UserActionEvent {
    Scroll(ScrollDirection),
    DeleteTable {
        table: TableSchema,
    },
    CreateTable {
        name: String,
    },
    CreateColumn {
        table: TableSchema,
        name: String,
        ty: ColumnType,
    },
    ViewTable {
        table: TableSchema,
    },
    Escape,
    NoAction,
}

/// Each page chooses what leaf event types to subscribe to.
pub enum AppEvent {
    UserAction(UserActionEvent),
    AsyncMessage(AppOperationResult),
    Tick,
}

/// An app state that can be projected onto a UI page. Note that the Exited state
/// cannot be projected.
pub trait Renderable: Sized {
    fn draw(&mut self, frame: &mut Frame);

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent>;

    /// Consumes the current page and produces the next `AppState` in response to a
    /// user action, either with the same page type with possibly mutated fields or
    /// of a different page type, along with an optional async operation request to
    /// dispatch as a result of the transition.
    fn next_state_from_user_action(
        self,
        action: &UserActionEvent,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)>;

    /// Consumes the current page and produces the next `AppState` in response to an
    /// async operation result.
    fn next_state_from_async_message(
        self,
        msg: &AppOperationResult,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)>;

    /// Consumes the current page and produces the next `AppState` in response to a
    /// clock tick.
    fn next_state_from_tick(self) -> anyhow::Result<(AppState, Option<AppOperationRequest>)>;

    /// Dispatches `app_event` to the appropriate `next_state_from_*` method.
    fn next_state_from_event(
        self,
        app_event: &AppEvent,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        match app_event {
            AppEvent::UserAction(action) => self.next_state_from_user_action(action),
            AppEvent::AsyncMessage(msg) => self.next_state_from_async_message(msg),
            AppEvent::Tick => self.next_state_from_tick(),
        }
    }
}

// TODO: make AppState UI-agnostic?
pub enum AppState {
    HomePage(HomePage),
    TablePage(TablePage),
    Exited,
}

impl AppState {
    pub fn draw(&mut self, frame: &mut Frame) {
        match self {
            AppState::HomePage(page) => page.draw(frame),
            AppState::TablePage(page) => page.draw(frame),
            AppState::Exited => (),
        }
    }

    pub async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent> {
        match self {
            AppState::HomePage(page) => page.collect_action(event_stream).await,
            AppState::TablePage(page) => page.collect_action(event_stream).await,
            AppState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }

    pub fn transition_app_state(
        self,
        app_event: &AppEvent,
        terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let current_kind = discriminant(&self);

        // Obtain next state and possible async requests=
        let (next_state, request) = match self {
            AppState::HomePage(page) => page.next_state_from_event(app_event),
            AppState::TablePage(page) => page.next_state_from_event(app_event),
            AppState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }?;

        // Clear terminal on page changes
        let next_kind = discriminant(&next_state);
        if next_kind != current_kind {
            terminal.clear().context("while clearing terminal")?;
        }

        Ok((next_state, request))
    }
}

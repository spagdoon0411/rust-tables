mod home_page;
mod table_page;

pub use home_page::HomePage;
pub use table_page::TablePage;

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

/// Actions that can be collected by a UI. Typically validated by a transaction/api
/// layer, but can be short-circuited (e.g., scrolling touches no persistent data).
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

// Each page chooses what leaf event types to subscribe to through transition_app_state.
// Irrelevant events are dropped, allowing a new page to discard obsolete async
// messages from an old page, for instance.
pub enum AppEvent {
    UserAction(UserActionEvent),
    AsyncMessage(AppOperationResult),
    Tick,
}

/// An app state that can be projected onto a UI page. Note that the Exited state
/// cannot be projected.
pub trait RenderableAppPage: Into<AppState> + Sized {
    fn draw(&mut self, frame: &mut Frame);

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent>;

    /// Consumes the current page and produces the next `AppState` in response to an
    /// event, either with the same page type with possibly mutated fields or of a
    /// different page type, along with an optional async operation request to
    /// dispatch as a result of the transition.
    fn derive_next_app_state(
        self,
        app_event: &AppEvent,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)>;
}

pub enum AppState {
    HomePage(HomePage),
    TablePage(TablePage),
    Exited,
}

impl RenderableAppPage for AppState {
    fn draw(&mut self, frame: &mut Frame) {
        match self {
            AppState::HomePage(page) => page.draw(frame),
            AppState::TablePage(page) => page.draw(frame),
            AppState::Exited => (),
        }
    }

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent> {
        match self {
            AppState::HomePage(page) => page.collect_action(event_stream).await,
            AppState::TablePage(page) => page.collect_action(event_stream).await,
            AppState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }

    fn derive_next_app_state(
        self,
        app_event: &AppEvent,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        match self {
            AppState::HomePage(page) => page.derive_next_app_state(app_event),
            AppState::TablePage(page) => page.derive_next_app_state(app_event),
            AppState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }
}

impl AppState {
    pub fn transition_app_state(
        self,
        app_event: &AppEvent,
        terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let current_kind = discriminant(&self);

        let (next_state, request) = self.derive_next_app_state(app_event)?;
        let next_kind = discriminant(&next_state);
        if next_kind != current_kind {
            terminal.clear().context("while clearing terminal")?;
        }
        Ok((next_state, request))
    }
}

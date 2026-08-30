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
    // Valid page type(s) this page can transition into
    type Next;

    fn draw(&mut self, frame: &mut Frame);

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent>;

    /// Consumes the current page and produces the next `Self::Next` in response to a
    /// user action, either with the same page type with possibly mutated fields or
    /// of a different page type, along with an optional async operation request to
    /// dispatch as a result of the transition.
    fn next_state_from_user_action(
        self,
        action: &UserActionEvent,
    ) -> anyhow::Result<(Self::Next, Option<AppOperationRequest>)>;

    /// Consumes the current page and produces the next `Self::Next` in response to an
    /// async operation result.
    fn next_state_from_async_message(
        self,
        msg: &AppOperationResult,
    ) -> anyhow::Result<(Self::Next, Option<AppOperationRequest>)>;

    /// Consumes the current page and produces the next `Self::Next` in response to a
    /// clock tick.
    fn next_state_from_tick(self) -> anyhow::Result<(Self::Next, Option<AppOperationRequest>)>;

    /// Dispatches `app_event` to the appropriate `next_state_from_*` method.
    fn next_state_from_event(
        self,
        app_event: &AppEvent,
    ) -> anyhow::Result<(Self::Next, Option<AppOperationRequest>)> {
        match app_event {
            AppEvent::UserAction(action) => self.next_state_from_user_action(action),
            AppEvent::AsyncMessage(msg) => self.next_state_from_async_message(msg),
            AppEvent::Tick => self.next_state_from_tick(),
        }
    }
}

// TODO: make AppState UI-agnostic?
pub enum PageState {
    HomePage(HomePage),
    TablePage(TablePage),
    Exited,
}

pub struct AppState {
    pub page_state: PageState,
}

impl Renderable for PageState {
    //
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

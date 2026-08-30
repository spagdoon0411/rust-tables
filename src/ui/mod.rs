mod app_state;
mod home_page;
mod notifications;
mod page_state;
mod table_page;

pub use app_state::AppState;
pub use home_page::HomePage;
pub use notifications::NotifListState;
pub use page_state::PageState;

use crossterm::event::EventStream;
use ratatui::Frame;

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

mod app_state;
mod home_page;
mod notifications;
mod page_state;
mod table_page;

use anyhow::Context;
pub use app_state::AppState;
pub use page_state::PageState;

use crossterm::{
    event::{
        EventStream, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::supports_keyboard_enhancement,
};
use ratatui::{DefaultTerminal, Frame};
use std::io::stdout;
use tokio::time::{self, Duration, Interval};

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

pub struct RatatuiUI {
    pub event_stream: EventStream,
    pub terminal: DefaultTerminal,
    pub tick: Interval,
    pub keyboard_enhancement: bool,
}

impl RatatuiUI {
    pub fn new() -> Self {
        RatatuiUI {
            event_stream: EventStream::new(),
            terminal: ratatui::init(),
            tick: time::interval(Duration::from_millis(100)),
            keyboard_enhancement: false,
        }
    }

    pub fn project(&mut self, app_state: &mut AppState) -> anyhow::Result<()> {
        self.terminal
            .draw(|frame| app_state.draw(frame))
            .context("drawing frame to terminal")?;
        Ok(())
    }

    /// Enables terminal-specific input/output enhancements where supported, returning
    /// whether they were enabled so the caller can undo them symmetrically on shutdown.
    pub fn init(&mut self) -> anyhow::Result<()> {
        // The Kitty keyboard protocol reports Escape unambiguously, letting supporting
        // terminals skip the escape-sequence disambiguation delay entirely.
        let keyboard_enhancement_supported = supports_keyboard_enhancement().unwrap_or(false);
        if keyboard_enhancement_supported {
            execute!(
                stdout(),
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )
            .context("pushing keyboard enhancement flags")?;
        }
        self.keyboard_enhancement = keyboard_enhancement_supported;
        Ok(())
    }

    /// Undoes the enhancements enabled by `terminal_specific_config`, given whether they
    /// were enabled.
    pub fn cleanup(&self) -> anyhow::Result<()> {
        if self.keyboard_enhancement {
            execute!(stdout(), PopKeyboardEnhancementFlags)
                .context("tearing down Kitty keyboard enhancements")?;
        }

        ratatui::restore();
        Ok(())
    }
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

use std::time::Duration;

use anyhow::Context;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use tokio::time::{self, Interval};
use tokio_stream::StreamExt;

use crate::ui::{Component, events::AppEvent};

/// Owns the terminal and the crossterm event stream, and drives the render/event loop
/// for a root [`Component`].
pub struct RatatuiUI {
    terminal: DefaultTerminal,
    events: EventStream,
    ticker: Interval,
    initialized: bool,
}

impl RatatuiUI {
    /// How often [`RatatuiUI::next_tick`] fires in the absence of any other event.
    /// Pushes animations and re-reads from a store that has possibly updated.
    pub const TICK_RATE: Duration = Duration::from_millis(250);

    pub fn new() -> Self {
        Self {
            terminal: ratatui::init(),
            events: EventStream::new(),
            ticker: time::interval(Self::TICK_RATE),
            initialized: false,
        }
    }

    /// Renders the root component's current state to the terminal.
    pub fn project(&mut self, app: &mut dyn Component) -> anyhow::Result<()> {
        self.terminal.draw(|frame| {
            app.render(frame, frame.area());
        })?;

        Ok(())
    }

    /// Reads terminal events until a key press resolves to an [`AppEvent`].
    async fn next_key_event(events: &mut EventStream) -> anyhow::Result<AppEvent> {
        loop {
            let event = events
                .next()
                .await
                .context("crossterm event stream ended")??;

            if let Event::Key(key) = event
                && key.kind == KeyEventKind::Press
            {
                return Ok(match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        AppEvent::Exit
                    }
                    code => AppEvent::KeyPress(code),
                });
            }
        }
    }

    /// Resolves to [`AppEvent::Tick`] on a fixed cadence ([`RatatuiUI::TICK_RATE`])
    async fn next_tick_event(ticker: &mut Interval) -> AppEvent {
        ticker.tick().await;
        AppEvent::Tick
    }

    pub async fn next_event(&mut self, on_exit: impl FnOnce()) -> anyhow::Result<AppEvent> {
        let event = if self.initialized {
            tokio::select! {
                event = Self::next_key_event(&mut self.events) => event?,
                event = Self::next_tick_event(&mut self.ticker) => event,
            }
        } else {
            self.initialized = true;
            AppEvent::Init
        };

        if let AppEvent::Exit = event {
            on_exit();
        }

        Ok(event)
    }
}

impl Drop for RatatuiUI {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

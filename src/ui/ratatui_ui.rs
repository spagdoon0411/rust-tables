use anyhow::Context;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use tokio_stream::StreamExt;

use crate::ui::{Component, events::AppEvent};

/// Owns the terminal and the crossterm event stream, and drives the render/event loop
/// for a root [`Component`].
pub struct RatatuiUI {
    terminal: DefaultTerminal,
    events: EventStream,
}

impl RatatuiUI {
    pub fn new() -> Self {
        Self {
            terminal: ratatui::init(),
            events: EventStream::new(),
        }
    }

    /// Renders the root component's current state to the terminal.
    pub fn project(&mut self, app: &mut dyn Component) -> anyhow::Result<()> {
        self.terminal.draw(|frame| {
            app.render(frame, frame.area());
        })?;

        Ok(())
    }

    // Collects events via Crossterm.
    pub async fn next_event(&mut self) -> anyhow::Result<AppEvent> {
        loop {
            let event = self
                .events
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
}

impl Drop for RatatuiUI {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

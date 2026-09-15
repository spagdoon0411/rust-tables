use anyhow::Context;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use tokio_stream::StreamExt;

#[derive(Clone, Copy)]
pub enum AppEvent {
    KeyPress(KeyCode),
    Exit,
}

pub async fn collect_crossterm_event(events: &mut EventStream) -> anyhow::Result<AppEvent> {
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

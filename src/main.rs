mod repository;
mod tables;
mod transactions;
mod ui;

use crate::{
    transactions::{AppOperationResult, launch},
    ui::{AppState, HomePage, RenderableAppPage},
};
use crossterm::{
    event::{
        EventStream, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::supports_keyboard_enhancement,
};
use sqlx::{Pool, Sqlite};
use std::{io::stdout, process::ExitCode};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use ui::AppEvent;

/// Enables terminal-specific input/output enhancements where supported, returning
/// whether they were enabled so the caller can undo them symmetrically on shutdown.
fn terminal_specific_config() -> anyhow::Result<bool> {
    // The Kitty keyboard protocol reports Escape unambiguously, letting supporting
    // terminals skip the escape-sequence disambiguation delay entirely.
    let keyboard_enhancement_supported = supports_keyboard_enhancement().unwrap_or(false);
    if keyboard_enhancement_supported {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }
    Ok(keyboard_enhancement_supported)
}

/// Undoes the enhancements enabled by `terminal_specific_config`, given whether they
/// were enabled.
fn terminal_specific_cleanup(keyboard_enhancement_supported: bool) -> anyhow::Result<()> {
    if keyboard_enhancement_supported {
        execute!(stdout(), PopKeyboardEnhancementFlags)?;
    }
    Ok(())
}

/// Primary application, evolving valid initial user data according to terminal input.
async fn app(pool: Pool<Sqlite>) -> anyhow::Result<()> {
    let mut event_stream = EventStream::new();
    let mut terminal = ratatui::init();
    let keyboard_enhancement_supported = terminal_specific_config()?;

    let mut app_state = AppState::HomePage(HomePage::new());
    let mut tick = time::interval(Duration::from_millis(100));
    let (tx, mut rx) = mpsc::channel::<AppOperationResult>(100);

    loop {
        // Terminal states are never projected to the terminal and never
        // initiate transitions.
        if matches!(app_state, AppState::Exited) {
            break;
        }

        // Project app state onto terminal
        terminal.draw(|frame| app_state.draw(frame))?;

        // Race receipts of AppEvents
        let app_event = tokio::select! {
            /* UserAction */ action = app_state.collect_action(&mut event_stream) => AppEvent::UserAction(action?),
            /* AsyncMessage */ Some(msg) = rx.recv() => AppEvent::AsyncMessage(msg),
            /* Tick */ _ = tick.tick() => AppEvent::Tick,
        };

        // Transition app state, clearing the terminal when necessary
        let request;
        (app_state, request) = app_state.transition_app_state(&app_event, &mut terminal)?;

        if let Some(request) = request {
            launch(tx.clone(), pool.clone(), request);
        }
    }

    terminal_specific_cleanup(keyboard_enhancement_supported)?;
    ratatui::restore(); // Return user's terminal to original state
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    // Verify initial user data state
    let pool = match repository::init_user_data().await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };

    // Application acts on valid user data
    match app(pool).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

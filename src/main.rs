mod repository;
mod tables;
mod transactions;
mod ui;

use crate::ui::{AppState, HomePage, RenderableAppPage};
use crossterm::event::EventStream;
use sqlx::{Pool, Sqlite};
use std::process::ExitCode;
use ui::AppEvent;

/// Primary application, evolving valid initial user data according to terminal input.
async fn app(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    let mut event_stream = EventStream::new();
    let mut terminal = ratatui::init();
    let mut app_state = AppState::HomePage(HomePage::new());

    loop {
        // Terminal states are never projected to the terminal and never
        // initiate transitions.
        if matches!(app_state, AppState::Exited) {
            break;
        }

        // Project app state onto terminal
        terminal.draw(|frame| app_state.draw(frame))?;

        // Poll for actions
        let app_event = AppEvent::UserAction(app_state.collect_action(&mut event_stream).await?);

        // Transition app state, clearing the terminal when necessary
        app_state = app_state.transition_app_state(&app_event, &mut terminal)?;
    }

    ratatui::restore(); // Return user's terminal to original state
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    // Verify initial user data state
    let db = match repository::init_user_data().await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };

    // Application acts on valid user data
    match app(&db).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

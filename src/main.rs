mod repository;
mod tables;
mod transactions;
mod ui;

use crate::ui::{AppState, HomePage, RenderableAppPage};
use sqlx::{Pool, Sqlite};
use std::process::ExitCode;

/// Primary application, evolving valid initial user data according to terminal input.
fn app(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
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

        // Poll for actions -- TODO: listen for events instead?
        let action = app_state.collect_action(&mut terminal)?;

        // TODO: route actions through transaction/api layer here for "approval"

        // Transition app state, clearing the terminal when necessary
        app_state = app_state.transition_app_state(&action, &mut terminal)?;
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
    match app(&db) {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

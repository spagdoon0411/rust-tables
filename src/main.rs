mod repository;
mod tables;
mod transactions;
mod ui;

use crate::{
    transactions::AsyncRequestStream,
    ui::{AppState, PageState, RatatuiUI, RenderRatatui},
};
use std::process::ExitCode;
use ui::AppEvent;

/// Primary application, evolving valid initial user data according to terminal input.
async fn app() -> anyhow::Result<()> {
    let mut ui = RatatuiUI::new();
    let mut app_state = AppState::new();
    let mut async_requests = AsyncRequestStream::new(repository::init_user_data().await?);

    ui.init()?;

    loop {
        // Terminal states are never projected to the terminal and never
        // initiate transitions.
        if matches!(app_state.page_state, PageState::Exited) {
            break;
        }

        // Project app state onto terminal
        ui.project(&mut app_state)?;

        // Race receipts of AppEvents
        let app_event = tokio::select! {
            /* UserAction */ action = app_state.collect_action(&mut ui.event_stream) => AppEvent::UserAction(action?),
            /* AsyncMessage */ Some(msg) = async_requests.recv() => AppEvent::AsyncMessage(msg),
            /* Tick */ _ = ui.tick.tick() => AppEvent::Tick,
        };

        // Transition app state, clearing the terminal when necessary
        let request;
        (app_state, request) = app_state.transition_app_state(&app_event, &mut ui.terminal)?;

        if let Some(request) = request {
            async_requests.execute_request(request);
        }
    }

    ui.cleanup()?;

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match app().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

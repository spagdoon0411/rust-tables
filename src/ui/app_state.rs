use super::AppStateTransition;
use super::RenderRatatui;
use super::UserActionEvent;
use super::home_page::HomePage;
use super::page_state::PageState;

use super::AppEvent;

use crate::transactions::{AppOperationRequest, AppOperationResult};
use crate::ui::notifications::NotifListState;

use anyhow::Context;
use ratatui::{DefaultTerminal, Frame};
use std::mem::discriminant;

pub struct AppState {
    pub page_state: PageState,
    pub notif_list_state: NotifListState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            page_state: PageState::HomePage(HomePage::new()),
            notif_list_state: NotifListState::new(),
        }
    }

    pub fn transition_app_state(
        self,
        app_event: &AppEvent,
        terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let current_page_kind = discriminant(&self.page_state);

        let (next_state, request) = self.next_state_from_event(app_event)?;

        // Clear terminal on page changes
        let next_page_kind = discriminant(&next_state.page_state);
        if next_page_kind != current_page_kind {
            terminal.clear().context("while clearing terminal")?;
        }

        Ok((next_state, request))
    }
}

impl RenderRatatui for AppState {
    fn draw(&mut self, frame: &mut Frame) {
        self.notif_list_state.draw(frame);
        self.page_state.draw(frame);
    }

    async fn collect_action(
        &mut self,
        event_stream: &mut crossterm::event::EventStream,
    ) -> anyhow::Result<UserActionEvent> {
        // TODO: collect actions from UI components using cursor focus target
        let action = self.page_state.collect_action(event_stream).await?;
        Ok(action)
    }
}

impl AppStateTransition for AppState {
    // The app's transitions are driven entirely by its inner page state.
    type Next = AppState;

    fn next_state_from_user_action(
        self,
        action: &UserActionEvent,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let notif_list_state = self.notif_list_state;
        let (page_state, request) = self.page_state.next_state_from_user_action(action)?;
        Ok((
            AppState {
                page_state,
                notif_list_state,
            },
            request,
        ))
    }

    fn next_state_from_async_message(
        self,
        msg: &AppOperationResult,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let notif_list_state = self.notif_list_state;
        let (page_state, request) = self.page_state.next_state_from_async_message(msg)?;
        Ok((
            AppState {
                page_state,
                notif_list_state,
            },
            request,
        ))
    }

    fn next_state_from_tick(self) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let notif_list_state = self.notif_list_state;
        let (page_state, request) = self.page_state.next_state_from_tick()?;
        Ok((
            AppState {
                page_state,
                notif_list_state,
            },
            request,
        ))
    }
}

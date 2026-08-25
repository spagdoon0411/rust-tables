use anyhow::Context;
use crossterm::event::EventStream;
use futures_util::StreamExt;
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Flex, Layout},
    widgets::Paragraph,
};

use crate::tables::TableId;
use crate::ui::{AppEvent, AppState, RenderableAppPage, ScrollDirection, UserActionEvent};

use super::home_page::HomePage;

pub struct TablePage {
    table_id: TableId,
    last_key: Option<KeyCode>,
}

impl TablePage {
    pub fn new(table_id: TableId) -> Self {
        Self {
            table_id,
            last_key: None,
        }
    }
}

impl RenderableAppPage for TablePage {
    fn draw(&mut self, frame: &mut Frame) {
        let id_text = format!("table page {}", self.table_id.0);
        let key_text = match self.last_key {
            Some(key) => format!("last key pressed: {key:?}"),
            None => "last key pressed: <none>".to_string(),
        };

        let [id_area, key_area] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
            .flex(Flex::Center)
            .areas(frame.area());

        frame.render_widget(
            Paragraph::new(id_text).alignment(Alignment::Center),
            id_area,
        );
        frame.render_widget(
            Paragraph::new(key_text).alignment(Alignment::Center),
            key_area,
        );
    }

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent> {
        let reading = event_stream
            .next()
            .await
            .context("event stream ended")?
            .context("reading terminal input")?;

        Ok(match reading {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.last_key = Some(key.code);
                match key.code {
                    KeyCode::Char('h') | KeyCode::Left => {
                        UserActionEvent::Scroll(ScrollDirection::Left)
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        UserActionEvent::Scroll(ScrollDirection::Down)
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        UserActionEvent::Scroll(ScrollDirection::Up)
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        UserActionEvent::Scroll(ScrollDirection::Right)
                    }
                    KeyCode::Esc => UserActionEvent::Escape,
                    _ => UserActionEvent::NoAction,
                }
            }
            _ => UserActionEvent::NoAction,
        })
    }

    fn derive_next_app_state(self, app_event: &AppEvent) -> anyhow::Result<AppState> {
        match app_event {
            AppEvent::UserAction(action) => match action {
                UserActionEvent::Escape => Ok(AppState::HomePage(HomePage::new())),
                _ => Ok(AppState::TablePage(self)),
            },
            AppEvent::AsyncMessage(_) => Ok(AppState::TablePage(self)),
            AppEvent::Tick => Ok(AppState::TablePage(self)),
        }
    }
}

impl From<TablePage> for AppState {
    fn from(page: TablePage) -> Self {
        AppState::TablePage(page)
    }
}

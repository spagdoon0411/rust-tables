use anyhow::Context;
use crossterm::event::EventStream;
use futures_util::StreamExt;
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Flex, Layout, Rect},
    widgets::{Block, List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::tables::{TableId, TableSchema};
use crate::ui::{AppEvent, AppState, RenderableAppPage, ScrollDirection, UserActionEvent};

use super::table_page::TablePage;

pub struct HomePage {
    tables: Vec<TableSchema>,
    list_state: ListState,
    selected: usize,
}

impl HomePage {
    pub fn new() -> Self {
        let tables = vec![
            TableSchema {
                id: TableId::new(),
                name: "table1".into(),
                columns: vec![],
            },
            TableSchema {
                id: TableId::new(),
                name: "table2".into(),
                columns: vec![],
            },
            TableSchema {
                id: TableId::new(),
                name: "table3".into(),
                columns: vec![],
            },
        ];

        let list_state =
            ListState::default().with_selected(if tables.is_empty() { None } else { Some(0) });

        Self {
            tables,
            list_state,
            selected: 0,
        }
    }

    // Determine the centered area for the table list's enclosing border,
    // capping the inner content height at half the screen.
    fn layout_list_area(frame_area: Rect, names: &[&str]) -> Rect {
        let max_list_height = frame_area.height.saturating_sub(2) / 2;
        let content_height = names.len() as u16;
        let list_height = content_height.min(max_list_height).max(1);
        let needs_scrollbar = content_height > max_list_height;

        let text_width = names.iter().map(|name| name.len()).max().unwrap_or(0) as u16 + 4;
        let width = (text_width + u16::from(needs_scrollbar) + 2).min(frame_area.width);

        let [vertical_area] = Layout::vertical([Constraint::Length(list_height + 2)])
            .flex(Flex::Center)
            .areas(frame_area);
        let [centered_area] = Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .areas(vertical_area);

        centered_area
    }

    // Render the table list inside a bordered block, splitting off a
    // scrollbar column when the list doesn't fit its allotted area.
    fn draw_list(frame: &mut Frame, area: Rect, names: Vec<&str>, state: &mut ListState) {
        let content_height = names.len() as u16;
        let list = List::new(names).highlight_symbol("> ");

        let block = Block::bordered();
        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        if inner_area.height < content_height {
            let [list_area, scrollbar_area] =
                Layout::horizontal([Constraint::Min(0), Constraint::Length(1)]).areas(inner_area);

            frame.render_stateful_widget(list, list_area, state);

            let mut scrollbar_state = ScrollbarState::new(content_height as usize)
                .position(state.selected().unwrap_or(0));
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                scrollbar_area,
                &mut scrollbar_state,
            );
        } else {
            frame.render_stateful_widget(list, inner_area, state);
        }
    }

    // Compose layout and rendering into a single draw call.
}

impl RenderableAppPage for HomePage {
    fn draw(&mut self, frame: &mut Frame) {
        let names: Vec<&str> = self
            .tables
            .iter()
            .map(|table| table.name.as_str())
            .collect();
        let list_area = Self::layout_list_area(frame.area(), &names);
        Self::draw_list(frame, list_area, names, &mut self.list_state);
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
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selected = self.selected.checked_sub(1).unwrap_or(self.selected);
                    UserActionEvent::Scroll(ScrollDirection::Up)
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.selected = (self.selected + 1).min(self.tables.len().saturating_sub(1));
                    UserActionEvent::Scroll(ScrollDirection::Down)
                }
                KeyCode::Enter => match self.tables.get(self.selected) {
                    Some(table) => UserActionEvent::ViewTable {
                        table: table.clone(),
                    },
                    None => UserActionEvent::NoAction,
                },
                KeyCode::Char('q') | KeyCode::Esc => UserActionEvent::Escape,
                _ => UserActionEvent::NoAction,
            },
            _ => UserActionEvent::NoAction,
        })
    }

    fn derive_next_app_state_for_event(mut self, app_event: &AppEvent) -> anyhow::Result<AppState> {
        match app_event {
            AppEvent::UserAction(action) => match action {
                UserActionEvent::Scroll(ScrollDirection::Up | ScrollDirection::Down) => {
                    self.list_state.select(Some(self.selected));
                    Ok(AppState::HomePage(self))
                }
                UserActionEvent::ViewTable { table } => {
                    Ok(AppState::TablePage(TablePage::new(table.id.clone())))
                }
                UserActionEvent::Escape => Ok(AppState::Exited),
                _ => Ok(AppState::HomePage(self)),
            },
            _ => todo!("async messages are not supported yet"),
        }
    }
}

impl From<HomePage> for AppState {
    fn from(page: HomePage) -> Self {
        AppState::HomePage(page)
    }
}

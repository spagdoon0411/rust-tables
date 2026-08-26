use anyhow::Context;
use crossterm::event::EventStream;
use futures_util::StreamExt;
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    widgets::{Block, List, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::transactions::{AppOperationRequest, AppOperationResult};
use crate::ui::{AppEvent, AppState, RenderableAppPage, ScrollDirection, UserActionEvent};
use crate::{tables::TableSchema, transactions::RetrieveTablesOutput};

use super::table_page::TablePage;

enum TableList {
    NotRequested,
    Loading,
    Loaded {
        tables: Vec<TableSchema>,
        list_state: ListState,
        selected: usize,
    },
}

pub struct HomePage {
    table_list: TableList,
}

impl HomePage {
    pub fn new() -> Self {
        Self {
            table_list: TableList::NotRequested,
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

    // Renders a centered "Loading tables..." message in place of the table
    // menu while no table list is available yet.
    fn draw_loading(frame: &mut Frame) {
        let [area] = Layout::vertical([Constraint::Length(1)])
            .flex(Flex::Center)
            .areas(frame.area());

        frame.render_widget(
            Paragraph::new("Loading tables...").alignment(Alignment::Center),
            area,
        );
    }
}

impl HomePage {
    fn respond_user_action(
        mut self,
        action: &UserActionEvent,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        match action {
            UserActionEvent::Scroll(ScrollDirection::Up | ScrollDirection::Down) => {
                if let TableList::Loaded {
                    list_state,
                    selected,
                    ..
                } = &mut self.table_list
                {
                    list_state.select(Some(*selected));
                }
                Ok((AppState::HomePage(self), None))
            }
            UserActionEvent::ViewTable { table } => {
                Ok((AppState::TablePage(TablePage::new(table.id.clone())), None))
            }
            UserActionEvent::Escape => Ok((AppState::Exited, None)),
            _ => Ok((AppState::HomePage(self), None)),
        }
    }

    fn respond_async_message(
        mut self,
        msg: &AppOperationResult,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        match msg {
            AppOperationResult::RetrieveTables(result) => match result {
                Ok(RetrieveTablesOutput { tables }) => {
                    let list_state = ListState::default().with_selected(if tables.is_empty() {
                        None
                    } else {
                        Some(0)
                    });

                    self.table_list = TableList::Loaded {
                        tables: tables.clone(),
                        list_state,
                        selected: 0,
                    };
                }
                Err(err) => {}
            },
            _ => { /* Subscribe only to RetrieveTables */ }
        }

        Ok((AppState::HomePage(self), None))
    }

    fn respond_tick(mut self) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        match self.table_list {
            TableList::NotRequested => {
                self.table_list = TableList::Loading;
                Ok((
                    AppState::HomePage(self),
                    Some(AppOperationRequest::RetrieveTables),
                ))
            }
            _ => Ok((AppState::HomePage(self), None)),
        }
    }

    // Handles input while no table list is available to select from; only
    // quitting is meaningful.
    fn collect_action_loading(reading: &Event) -> UserActionEvent {
        match reading {
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) =>
            {
                UserActionEvent::Escape
            }
            _ => UserActionEvent::NoAction,
        }
    }

    // Handles input against a loaded table list, tracking the selected row.
    fn collect_action_loaded(
        tables: &[TableSchema],
        selected: &mut usize,
        reading: &Event,
    ) -> UserActionEvent {
        match reading {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('k') | KeyCode::Up => {
                    *selected = selected.checked_sub(1).unwrap_or(*selected);
                    UserActionEvent::Scroll(ScrollDirection::Up)
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    *selected = (*selected + 1).min(tables.len().saturating_sub(1));
                    UserActionEvent::Scroll(ScrollDirection::Down)
                }
                KeyCode::Enter => match tables.get(*selected) {
                    Some(table) => UserActionEvent::ViewTable {
                        table: table.clone(),
                    },
                    None => UserActionEvent::NoAction,
                },
                KeyCode::Char('q') | KeyCode::Esc => UserActionEvent::Escape,
                _ => UserActionEvent::NoAction,
            },
            _ => UserActionEvent::NoAction,
        }
    }
}

impl RenderableAppPage for HomePage {
    fn draw(&mut self, frame: &mut Frame) {
        match &mut self.table_list {
            TableList::NotRequested | TableList::Loading => Self::draw_loading(frame),
            TableList::Loaded {
                tables, list_state, ..
            } => {
                let names: Vec<&str> = tables.iter().map(|table| table.name.as_str()).collect();
                let list_area = Self::layout_list_area(frame.area(), &names);
                Self::draw_list(frame, list_area, names, list_state);
            }
        }
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

        Ok(match &mut self.table_list {
            TableList::Loaded {
                tables, selected, ..
            } => Self::collect_action_loaded(tables, selected, &reading),
            TableList::NotRequested | TableList::Loading => Self::collect_action_loading(&reading),
        })
    }

    fn derive_next_app_state(
        self,
        app_event: &AppEvent,
    ) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        match app_event {
            AppEvent::UserAction(action) => self.respond_user_action(action),
            AppEvent::AsyncMessage(msg) => self.respond_async_message(msg),
            AppEvent::Tick => self.respond_tick(),
        }
    }
}

impl From<HomePage> for AppState {
    fn from(page: HomePage) -> Self {
        AppState::HomePage(page)
    }
}

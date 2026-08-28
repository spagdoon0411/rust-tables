use anyhow::Context;
use crossterm::event::EventStream;
use futures_util::StreamExt;
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    widgets::{
        Block, List, ListState, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::transactions::{
    AppOperationRequest, AppOperationResult, CreateTableInput, DeleteTableInput,
};
use crate::ui::{AppState, RenderableAppPage, ScrollDirection, UserActionEvent};
use crate::{tables::TableSchema, transactions::RetrieveTablesOutput};

use super::table_page::TablePage;

// Displayed to the right of the table list, left-aligned with it: each
// available key alongside a brief description of what it does.
const KEY_HINTS: [(&str, &str); 4] = [
    ("j/k ↓/↑", "Scroll"),
    ("q/esc", "Return"),
    ("d", "Delete"),
    ("c", "New table"),
];

// The table list's row count is clamped to this range, so the box stays a
// roughly consistent size regardless of how many tables there are.
const MIN_LIST_HEIGHT: u16 = 16;
const MAX_LIST_HEIGHT: u16 = 16;

// The table list's column count is clamped to this range, so the box stays a
// roughly consistent size regardless of table name lengths.
const MIN_LIST_WIDTH: u16 = 64;
const MAX_LIST_WIDTH: u16 = 128;

enum TableList {
    NotRequested,
    Loading,
    Loaded {
        tables: Vec<TableSchema>,
        list_state: ListState,
        selected: usize,
    },
}

enum CreationMenu {
    ViewingList,
    CreatingTable(String),
}

pub struct HomePage {
    table_list: TableList,
    creating: CreationMenu,
}

impl HomePage {
    pub fn new() -> Self {
        Self {
            table_list: TableList::NotRequested,
            creating: CreationMenu::ViewingList,
        }
    }

    // Formats each key hint as "<key>  <description>", padding keys to a
    // common width so the descriptions line up.
    fn key_hint_lines() -> Vec<String> {
        let key_width = KEY_HINTS
            .iter()
            .map(|(key, _)| key.len())
            .max()
            .unwrap_or(0);

        KEY_HINTS
            .iter()
            .map(|(key, description)| format!("{key:key_width$}  {description}"))
            .collect()
    }

    // Determine the centered areas for the table list's enclosing border and
    // its key hints, the latter placed to the right and left-aligned with
    // the list (but one row lower). The list's row and column counts are
    // clamped to [MIN_LIST_HEIGHT, MAX_LIST_HEIGHT] and
    // [MIN_LIST_WIDTH, MAX_LIST_WIDTH] respectively, so the box stays a
    // roughly consistent size regardless of the table list's contents.
    // When `show_input` is set, an extra bordered row is reserved directly
    // under the list (same width) for the create-table input, returned as
    // the third element.
    fn layout_list_area(
        frame_area: Rect,
        names: &[&str],
        show_input: bool,
    ) -> (Rect, Rect, Option<Rect>) {
        let content_height = names.len() as u16;
        let list_height = content_height.clamp(MIN_LIST_HEIGHT, MAX_LIST_HEIGHT);
        let needs_scrollbar = content_height > list_height;

        let text_width = names.iter().map(|name| name.len()).max().unwrap_or(0) as u16 + 4;
        let list_width =
            (text_width + u16::from(needs_scrollbar) + 2).clamp(MIN_LIST_WIDTH, MAX_LIST_WIDTH);

        let hint_lines = Self::key_hint_lines();
        let hint_gap = 2u16;
        let hint_width = hint_lines.iter().map(|line| line.len()).max().unwrap_or(0) as u16;

        let total_width = list_width + hint_gap + hint_width;
        let input_height = if show_input { 3u16 } else { 0u16 };

        let [vertical_area] =
            Layout::vertical([Constraint::Length(list_height + 2 + input_height)])
                .flex(Flex::Center)
                .areas(frame_area);
        let [centered_area] = Layout::horizontal([Constraint::Length(total_width)])
            .flex(Flex::Center)
            .areas(vertical_area);

        let [top_area, input_area] = Layout::vertical([
            Constraint::Length(list_height + 2),
            Constraint::Length(input_height),
        ])
        .areas(centered_area);

        let [list_area, _gap_area, hints_area] = Layout::horizontal([
            Constraint::Length(list_width),
            Constraint::Length(hint_gap),
            Constraint::Length(hint_width),
        ])
        .areas(top_area);

        let hints_area = Rect {
            y: hints_area.y + 1,
            height: hints_area.height.saturating_sub(1),
            ..hints_area
        };

        let input_area = show_input.then_some(Rect {
            width: list_width,
            ..input_area
        });

        (list_area, hints_area, input_area)
    }

    // Render the table list inside a bordered block, splitting off a
    // scrollbar column when the list doesn't fit its allotted area.
    fn draw_list(frame: &mut Frame, area: Rect, names: Vec<&str>, state: &mut ListState) {
        let content_height = names.len() as u16;
        let list = List::new(names).highlight_symbol("> ");

        let block = Block::bordered().padding(Padding::horizontal(1));
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

    // Renders the key hints, left-aligned, top-aligned with the list.
    fn draw_key_hints(frame: &mut Frame, area: Rect) {
        let text = Self::key_hint_lines().join("\n");
        frame.render_widget(Paragraph::new(text).alignment(Alignment::Left), area);
    }

    // Renders the new-table name input, directly under the list.
    fn draw_create_input(frame: &mut Frame, area: Rect, buffer: &str) {
        let block = Block::bordered()
            .title(" New table name ")
            .padding(Padding::new(1, 0, 0, 0));
        let text = format!("{buffer}\u{2588}");
        frame.render_widget(Paragraph::new(text).block(block), area);
    }
}

impl HomePage {
    // Handles input while no table list is available to select from; only
    // quitting is meaningful.
    fn collect_action_loading(key_code: KeyCode) -> UserActionEvent {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc => UserActionEvent::Escape,
            _ => UserActionEvent::NoAction,
        }
    }

    // Handles input against a loaded table list, tracking the selected row.
    fn collect_action_loaded(
        tables: &[TableSchema],
        selected: &mut usize,
        key_code: KeyCode,
    ) -> UserActionEvent {
        match key_code {
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
            KeyCode::Char('d') => match tables.get(*selected) {
                Some(table) => UserActionEvent::DeleteTable {
                    table: table.clone(),
                },
                None => UserActionEvent::NoAction,
            },
            KeyCode::Char('q') | KeyCode::Esc => UserActionEvent::Escape,
            _ => UserActionEvent::NoAction,
        }
    }

    // Handles input while the create-table input is focused: characters are
    // appended to the buffer, Backspace removes the last character, Enter
    // confirms, and Esc cancels back to the table list.
    fn collect_action_creating(creating: &mut CreationMenu, key_code: KeyCode) -> UserActionEvent {
        let CreationMenu::CreatingTable(buffer) = creating else {
            return UserActionEvent::NoAction;
        };

        match key_code {
            KeyCode::Enter => {
                let name = std::mem::take(buffer);
                *creating = CreationMenu::ViewingList;
                UserActionEvent::CreateTable { name }
            }
            KeyCode::Esc => {
                *creating = CreationMenu::ViewingList;
                UserActionEvent::NoAction
            }
            KeyCode::Backspace => {
                buffer.pop();
                UserActionEvent::NoAction
            }
            KeyCode::Char(c) => {
                buffer.push(c);
                UserActionEvent::NoAction
            }
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
                let show_input = matches!(self.creating, CreationMenu::CreatingTable(_));
                let (list_area, hints_area, input_area) =
                    Self::layout_list_area(frame.area(), &names, show_input);
                Self::draw_list(frame, list_area, names, list_state);
                Self::draw_key_hints(frame, hints_area);
                if let (Some(area), CreationMenu::CreatingTable(buffer)) =
                    (input_area, &self.creating)
                {
                    Self::draw_create_input(frame, area, buffer);
                }
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

        let Event::Key(key) = reading else {
            return Ok(UserActionEvent::NoAction);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(UserActionEvent::NoAction);
        }

        if matches!(self.creating, CreationMenu::CreatingTable(_)) {
            return Ok(Self::collect_action_creating(&mut self.creating, key.code));
        }

        Ok(match &mut self.table_list {
            TableList::Loaded {
                tables, selected, ..
            } => {
                if key.code == KeyCode::Char('c') {
                    self.creating = CreationMenu::CreatingTable(String::new());
                    UserActionEvent::NoAction
                } else {
                    Self::collect_action_loaded(tables, selected, key.code)
                }
            }
            TableList::NotRequested | TableList::Loading => Self::collect_action_loading(key.code),
        })
    }

    fn next_state_from_user_action(
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
            UserActionEvent::DeleteTable { table } => Ok((
                AppState::HomePage(self),
                Some(AppOperationRequest::DeleteTable(DeleteTableInput {
                    table_id: table.id.clone(),
                })),
            )),
            UserActionEvent::CreateTable { name } => Ok((
                AppState::HomePage(self),
                Some(AppOperationRequest::CreateTable(CreateTableInput {
                    name: name.clone(),
                })),
            )),
            UserActionEvent::Escape => Ok((AppState::Exited, None)),
            _ => Ok((AppState::HomePage(self), None)),
        }
    }

    fn next_state_from_async_message(
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
                Err(err) => {
                    todo!("Handle this error through a new UI object.")
                }
            },
            AppOperationResult::DeleteTable(result) => match result {
                Ok(_) => {
                    // Re-fetch the table list on the next tick.
                    self.table_list = TableList::NotRequested;
                }
                Err(err) => {
                    todo!("Handle this error through a new UI object.")
                }
            },
            AppOperationResult::CreateTable(result) => match result {
                Ok(_) => {
                    // Re-fetch the table list on the next tick.
                    self.table_list = TableList::NotRequested;
                }
                Err(err) => {
                    todo!("Handle this error through a new UI object.")
                }
            },
        }

        Ok((AppState::HomePage(self), None))
    }

    fn next_state_from_tick(mut self) -> anyhow::Result<(AppState, Option<AppOperationRequest>)> {
        let TableList::NotRequested = self.table_list else {
            return Ok((AppState::HomePage(self), None));
        };

        self.table_list = TableList::Loading;
        Ok((
            AppState::HomePage(self),
            Some(AppOperationRequest::RetrieveTables),
        ))
    }
}

impl From<HomePage> for AppState {
    fn from(page: HomePage) -> Self {
        AppState::HomePage(page)
    }
}

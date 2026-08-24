use std::mem::discriminant;

use anyhow::Context;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    widgets::{Block, List, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::tables::{ColumnType, TableId, TableSchema};

pub enum ScrollDirection {
    Left,
    Down,
    Up,
    Right,
}

/// Actions that can be collected by a UI. Typically validated by a transaction/api
/// layer, but can be short-circuited (e.g., scrolling touches no persistent data).
pub enum UserAction {
    Scroll(ScrollDirection),
    DeleteTable {
        table: TableSchema,
    },
    CreateTable {
        name: String,
    },
    CreateColumn {
        table: TableSchema,
        name: String,
        ty: ColumnType,
    },
    ViewTable {
        table: TableSchema,
    },
    Escape,
    NoAction,
}

pub trait RenderableAppPage {
    fn draw(&mut self, frame: &mut Frame);
    fn collect_action(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<UserAction>;

    /// Consumes the current page and produces the next `AppState`, either with the same
    /// page type with possibly mutated fields or of a different page type.
    fn derive_next_app_state(self, action: &UserAction) -> anyhow::Result<AppState>;
}

pub enum AppState {
    HomePage(HomePage),
    TablePage(TablePage),
    Exited,
}

impl RenderableAppPage for AppState {
    fn draw(&mut self, frame: &mut Frame) {
        match self {
            AppState::HomePage(page) => page.draw(frame),
            AppState::TablePage(page) => page.draw(frame),
            AppState::Exited => (),
        }
    }

    fn collect_action(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<UserAction> {
        match self {
            AppState::HomePage(page) => page.collect_action(terminal),
            AppState::TablePage(page) => page.collect_action(terminal),
            AppState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }

    fn derive_next_app_state(self, action: &UserAction) -> anyhow::Result<AppState> {
        match self {
            AppState::HomePage(page) => page.derive_next_app_state(action),
            AppState::TablePage(page) => page.derive_next_app_state(action),
            AppState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }
}

impl AppState {
    pub fn transition_app_state(
        self,
        action: &UserAction,
        terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<AppState> {
        let current_kind = discriminant(&self);
        let next_state = self.derive_next_app_state(action)?;
        let next_kind = discriminant(&next_state);
        if next_kind != current_kind {
            terminal.clear().context("while clearing terminal")?;
        }
        Ok(next_state)
    }
}

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

    fn collect_action(&mut self, _terminal: &mut DefaultTerminal) -> anyhow::Result<UserAction> {
        let reading = event::read().context("reading terminal input")?;

        Ok(match reading {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selected = self.selected.checked_sub(1).unwrap_or(self.selected);
                    UserAction::Scroll(ScrollDirection::Up)
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.selected = (self.selected + 1).min(self.tables.len().saturating_sub(1));
                    UserAction::Scroll(ScrollDirection::Down)
                }
                KeyCode::Enter => match self.tables.get(self.selected) {
                    Some(table) => UserAction::ViewTable {
                        table: table.clone(),
                    },
                    None => UserAction::NoAction,
                },
                KeyCode::Esc => UserAction::Escape,
                _ => UserAction::NoAction,
            },
            _ => UserAction::NoAction,
        })
    }

    fn derive_next_app_state(mut self, action: &UserAction) -> anyhow::Result<AppState> {
        match action {
            UserAction::Scroll(ScrollDirection::Up | ScrollDirection::Down) => {
                self.list_state.select(Some(self.selected));
                Ok(AppState::HomePage(self))
            }
            UserAction::ViewTable { table } => {
                Ok(AppState::TablePage(TablePage::new(table.id.clone())))
            }
            UserAction::Escape => Ok(AppState::Exited),
            _ => Ok(AppState::HomePage(self)),
        }
    }
}

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

    fn collect_action(&mut self, _terminal: &mut DefaultTerminal) -> anyhow::Result<UserAction> {
        let reading = event::read().context("reading terminal input")?;

        Ok(match reading {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.last_key = Some(key.code);
                match key.code {
                    KeyCode::Char('h') | KeyCode::Left => UserAction::Scroll(ScrollDirection::Left),
                    KeyCode::Char('j') | KeyCode::Down => UserAction::Scroll(ScrollDirection::Down),
                    KeyCode::Char('k') | KeyCode::Up => UserAction::Scroll(ScrollDirection::Up),
                    KeyCode::Char('l') | KeyCode::Right => {
                        UserAction::Scroll(ScrollDirection::Right)
                    }
                    KeyCode::Esc => UserAction::Escape,
                    _ => UserAction::NoAction,
                }
            }
            _ => UserAction::NoAction,
        })
    }

    fn derive_next_app_state(self, action: &UserAction) -> anyhow::Result<AppState> {
        Ok(match action {
            UserAction::Escape => AppState::HomePage(HomePage::new()),
            _ => AppState::TablePage(self),
        })
    }
}

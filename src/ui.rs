use std::mem::discriminant;

use anyhow::Context;
use crossterm::event::EventStream;
use futures_util::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{Event, KeyCode, KeyEventKind},
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
pub enum UserActionEvent {
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

pub enum AsyncMessageEvent {}

// Each page chooses what leaf event types to subscribe to through transition_app_state.
// Irrelevant events are dropped, allowing a new page to discard obsolete async
// messages from an old page, for instance.
pub enum AppEvent {
    UserAction(UserActionEvent),
    AsyncMessage(AsyncMessageEvent),
    Tick,
}

/// An app state that can be projected onto a UI page. Note that the Exited state
/// cannot be projected.
pub trait RenderableAppPage: Into<AppState> + Sized {
    fn draw(&mut self, frame: &mut Frame);

    /// Default tick behavior is consumption without side effects. Override this function
    /// on a page to produce more dynamic behavior.
    fn on_tick(self) -> anyhow::Result<AppState> {
        Ok(self.into())
    }

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent>;

    /// Consumes the current page and produces the next `AppState` in response to a
    /// non-tick event, either with the same page type with possibly mutated fields or
    /// of a different page type.
    fn derive_next_app_state_for_event(self, app_event: &AppEvent) -> anyhow::Result<AppState>;

    /// Single point of maintenance for tick vs. non-tick dispatch. Override `on_tick` on
    /// a page for custom tick behavior rather than overriding this.
    fn derive_next_app_state(self, app_event: &AppEvent) -> anyhow::Result<AppState> {
        match app_event {
            AppEvent::Tick => self.on_tick(),
            _ => self.derive_next_app_state_for_event(app_event),
        }
    }
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

    async fn collect_action(
        &mut self,
        event_stream: &mut EventStream,
    ) -> anyhow::Result<UserActionEvent> {
        match self {
            AppState::HomePage(page) => page.collect_action(event_stream).await,
            AppState::TablePage(page) => page.collect_action(event_stream).await,
            AppState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }

    fn on_tick(self) -> anyhow::Result<AppState> {
        match self {
            AppState::HomePage(page) => page.on_tick(),
            AppState::TablePage(page) => page.on_tick(),
            AppState::Exited => Ok(AppState::Exited),
        }
    }

    fn derive_next_app_state_for_event(self, app_event: &AppEvent) -> anyhow::Result<AppState> {
        match self {
            AppState::HomePage(page) => page.derive_next_app_state_for_event(app_event),
            AppState::TablePage(page) => page.derive_next_app_state_for_event(app_event),
            AppState::Exited => anyhow::bail!("should not have encountered Exited app state"),
        }
    }
}

impl From<HomePage> for AppState {
    fn from(page: HomePage) -> Self {
        AppState::HomePage(page)
    }
}

impl From<TablePage> for AppState {
    fn from(page: TablePage) -> Self {
        AppState::TablePage(page)
    }
}

impl AppState {
    pub fn transition_app_state(
        self,
        app_event: &AppEvent,
        terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<AppState> {
        let current_kind = discriminant(&self);

        let next_state = self.derive_next_app_state(app_event)?;
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
                KeyCode::Esc => UserActionEvent::Escape,
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

    fn derive_next_app_state_for_event(self, app_event: &AppEvent) -> anyhow::Result<AppState> {
        match app_event {
            AppEvent::UserAction(action) => match action {
                UserActionEvent::Escape => Ok(AppState::HomePage(HomePage::new())),
                _ => Ok(AppState::TablePage(self)),
            },
            _ => todo!("async messages are not supported yet"),
        }
    }
}

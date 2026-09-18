use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{
        Block, List, ListState, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crossterm::event::KeyCode;

use crate::{
    clients::{AsyncOperationRequest, TableSchema},
    dispatcher::Action,
    store::Loadable,
    ui::{Component, SharedContext, events::AppEvent},
};

/// Half-tone dots (░), used to shade a region while its backing data is still loading.
const HALFTONE_SHADE: &str = "░";

/// Fills `area` with a solid half-tone shade.
fn render_halftone_shade(frame: &mut Frame, area: Rect) {
    let line = HALFTONE_SHADE.repeat(area.width as usize);
    let lines = vec![Line::raw(line); area.height as usize];

    frame.render_widget(Paragraph::new(lines), area);
}

/// Centers a `width` x `height` rect within `area`, capping it to `area`'s size.
fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}

struct TablePreview {
    context: Arc<SharedContext>,
}

impl TablePreview {
    fn new(context: Arc<SharedContext>) -> Self {
        Self { context }
    }

    fn render_placeholder(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(Line::from(vec![Span::raw("─"), Span::raw("Preview")]));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        render_halftone_shade(frame, inner);
    }

    fn render_loaded(&mut self, frame: &mut Frame, area: Rect) {
        let name = self.context.store.with(|store| match &store.table_list {
            Loadable::Loaded(list) => list
                .selected()
                .map(|table| table.name.clone())
                .unwrap_or_default(),
            _ => unreachable!("TablePreview's loaded variant rendered before table list"),
        });

        let block = Block::bordered().title(Line::from(vec![Span::raw("─"), Span::raw("Preview")]));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(inner);

        let name = Paragraph::new(name).alignment(Alignment::Center);
        frame.render_widget(name, middle);
    }
}

impl Component for TablePreview {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let is_loaded = self
            .context
            .store
            .with(|store| matches!(store.table_list, Loadable::Loaded(_)));

        if is_loaded {
            self.render_loaded(frame, area);
        } else {
            self.render_placeholder(frame, area);
        }
    }

    fn handle_event(&mut self, _event: AppEvent) {}

    fn children(&mut self) -> Option<Vec<&mut dyn Component>> {
        None
    }
}

struct TableList {
    context: Arc<SharedContext>,
    list_state: ListState,
}

impl TableList {
    fn new(context: Arc<SharedContext>) -> Self {
        Self {
            context,
            list_state: ListState::default(),
        }
    }

    fn render_placeholder(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(Line::from(vec![Span::raw("─"), Span::raw("Tables")]))
            .padding(Padding::left(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        render_halftone_shade(frame, inner);

        // Rendered after the shade so its background stays blank instead of half-toned.
        let loading = "Loading...";
        let loading_box = centered_rect(inner, loading.len() as u16 + 4, 3);
        let loading_block = Block::bordered();
        let loading_inner = loading_block.inner(loading_box);
        frame.render_widget(loading_block, loading_box);
        frame.render_widget(
            Paragraph::new(loading).alignment(Alignment::Center),
            loading_inner,
        );
    }

    fn render_loaded(&mut self, frame: &mut Frame, area: Rect, tables: &[TableSchema]) {
        let block = Block::bordered()
            .title(Line::from(vec![Span::raw("─"), Span::raw("Tables")]))
            .padding(Padding::left(1));
        let inner = block.inner(area);
        let visible_lines = inner.height as usize;

        let list = List::new(tables.iter().map(|table| table.name.clone()))
            .block(block)
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.list_state);

        // Rendering the list above updates list_state's scroll offset to keep the
        // selection in view, so it's read back here to drive the scrollbar's position.
        if tables.len() > visible_lines {
            let mut scrollbar_state = ScrollbarState::new(tables.len() - visible_lines + 1)
                .position(self.list_state.offset());

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }
}

impl Component for TableList {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let selection = self.context.store.with(|store| match &store.table_list {
            Loadable::Loaded(list) => Some((list.items().to_vec(), list.selected_index())),
            _ => None,
        });

        match selection {
            Some((tables, selected)) => {
                self.list_state.select(selected);
                self.render_loaded(frame, area, &tables);
            }
            None => self.render_placeholder(frame, area),
        }
    }

    fn handle_event(&mut self, _event: AppEvent) {}

    fn children(&mut self) -> Option<Vec<&mut dyn Component>> {
        None
    }
}

pub struct TableBrowser {
    context: Arc<SharedContext>,
    table_list: TableList,
    table_preview: TablePreview,
}

impl TableBrowser {
    pub const MAX_WIDTH: u16 = 80;
    pub const MAX_HEIGHT: u16 = 16;

    pub fn new(context: Arc<SharedContext>) -> Self {
        let table_list = TableList::new(Arc::clone(&context));
        let table_preview = TablePreview::new(Arc::clone(&context));

        Self {
            context,
            table_list,
            table_preview,
        }
    }

    fn handle_init(&self) {
        self.context
            .request_tx
            .try_send(AsyncOperationRequest::ListTables)
            .expect("unable to send initialization requests")
    }

    fn emit_action(&self, action: Action) {
        self.context
            .action_tx
            .try_send(action)
            .expect("unable to send dispatched action")
    }

    fn handle_key_press(&self, key: KeyCode) {
        match key {
            KeyCode::Down | KeyCode::Char('j') => self.emit_action(Action::SelectNextTable),
            KeyCode::Up | KeyCode::Char('k') => self.emit_action(Action::SelectPreviousTable),
            _ => {}
        }
    }
}

impl Component for TableBrowser {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let width = area.width.min(Self::MAX_WIDTH);
        let height = area.height.min(Self::MAX_HEIGHT);

        let bounded = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(bounded);

        self.table_list.render(frame, left);
        self.table_preview.render(frame, right);
    }

    fn children(&mut self) -> Option<Vec<&mut dyn Component>> {
        Some(vec![&mut self.table_list, &mut self.table_preview])
    }

    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Init => self.handle_init(),
            AppEvent::KeyPress(key) => self.handle_key_press(key),
            _ => {}
        }
    }
}

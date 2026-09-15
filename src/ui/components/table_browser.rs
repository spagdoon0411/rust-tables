use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{
        Block, List, ListState, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::ui::{Component, events::AppEvent};

pub struct TableBrowser {
    items: Vec<String>,
    list_state: ListState,
}

impl TableBrowser {
    pub const MAX_WIDTH: u16 = 80;
    pub const MAX_HEIGHT: u16 = 16;

    pub fn new(items: Vec<String>) -> Self {
        let list_state = if items.is_empty() {
            ListState::default()
        } else {
            ListState::default().with_selected(Some(0))
        };

        Self { items, list_state }
    }

    /// The name of the currently selected table, if any.
    fn selected(&self) -> Option<&str> {
        self.list_state
            .selected()
            .and_then(|i| self.items.get(i))
            .map(String::as_str)
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        // Shift the title right by one column, filling the vacated column with the
        // border's own horizontal glyph so it reads as a continuation of the border
        // rather than a gap.
        let block = Block::bordered()
            .title(Line::from(vec![Span::raw("─"), Span::raw("Tables")]))
            .padding(Padding::left(1));
        let visible_lines = block.inner(area).height as usize;

        let list = List::new(self.items.iter().cloned())
            .block(block)
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.list_state);

        // Rendering the list above updates list_state's scroll offset to keep the
        // selection in view, so it's read back here to drive the scrollbar's position.
        if self.items.len() > visible_lines {
            let mut scrollbar_state = ScrollbarState::new(self.items.len() - visible_lines + 1)
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

    fn render_preview(&mut self, frame: &mut Frame, area: Rect) {
        // Shift the title right by one column, filling the vacated column with the
        // border's own horizontal glyph so it reads as a continuation of the border
        // rather than a gap.
        let block = Block::bordered().title(Line::from(vec![Span::raw("─"), Span::raw("Preview")]));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(inner);

        let name = Paragraph::new(self.selected().unwrap_or_default()).alignment(Alignment::Center);
        frame.render_widget(name, middle);
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

        self.render_list(frame, left);
        self.render_preview(frame, right);
    }

    fn children(&mut self) -> Option<Vec<&mut dyn Component>> {
        None
    }

    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::KeyPress(key) => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.list_state.select_previous();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.list_state.select_next();
                }
                _ => {}
            },
            // main's event loop intercepts Exit before it ever reaches a component.
            AppEvent::Exit => {}
        }
    }
}

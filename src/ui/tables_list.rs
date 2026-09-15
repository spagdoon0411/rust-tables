use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::{Line, Span},
    widgets::{Block, List, ListState, Padding, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::ui::Component;

pub struct TablesList {
    items: Vec<String>,
    list_state: ListState,
}

impl TablesList {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            list_state: ListState::default(),
        }
    }

    /// The name of the currently selected table, if any.
    pub fn selected(&self) -> Option<&str> {
        self.list_state
            .selected()
            .and_then(|i| self.items.get(i))
            .map(String::as_str)
    }
}

impl Component for TablesList {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
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
            let mut scrollbar_state =
                ScrollbarState::new(self.items.len()).position(self.list_state.offset());

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

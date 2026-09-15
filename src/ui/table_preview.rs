use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::ui::Component;

pub struct TablePreview {
    name: String,
}

impl TablePreview {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
}

impl Component for TablePreview {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
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

        let name = Paragraph::new(self.name.as_str()).alignment(Alignment::Center);
        frame.render_widget(name, middle);
    }
}

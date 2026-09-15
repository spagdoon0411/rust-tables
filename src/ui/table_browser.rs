use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

use crate::ui::{Component, TablePreview, TablesList};

pub struct TableBrowser {
    list: TablesList,
    preview: TablePreview,
}

impl TableBrowser {
    pub const MAX_WIDTH: u16 = 80;
    pub const MAX_HEIGHT: u16 = 16;

    pub fn new(items: Vec<String>) -> Self {
        Self {
            list: TablesList::new(items),
            preview: TablePreview::new(String::new()),
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

        if let Some(name) = self.list.selected() {
            self.preview.set_name(name);
        }

        self.list.render(frame, left);
        self.preview.render(frame, right);
    }
}

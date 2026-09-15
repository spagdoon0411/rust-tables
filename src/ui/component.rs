use ratatui::{Frame, layout::Rect};

/// A piece of UI that knows how to draw itself into a given area of the frame.
pub trait Component {
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

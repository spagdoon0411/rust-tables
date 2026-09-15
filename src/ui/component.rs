use ratatui::{Frame, layout::Rect};

use crate::events::AppEvent;

/// A piece of UI that knows how to draw itself into a given area of the frame.
pub trait Component {
    fn render(&mut self, frame: &mut Frame, area: Rect);
    fn children(&mut self) -> Option<Vec<&mut dyn Component>>;
    fn handle_event(&mut self, event: AppEvent);
    fn propagate_event(&mut self, event: AppEvent) {
        if let Some(children) = self.children() {
            for child in children {
                child.handle_event(event)
            }
        }

        self.handle_event(event);
    }
}

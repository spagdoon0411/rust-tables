use crossterm::event::KeyCode;

#[derive(Clone, Copy)]
pub enum AppEvent {
    KeyPress(KeyCode),
    Exit,
}

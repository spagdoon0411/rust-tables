use crossterm::event::KeyCode;

#[derive(Clone, Copy)]
pub enum AppEvent {
    Init,
    KeyPress(KeyCode),
    Tick,
    Exit,
}

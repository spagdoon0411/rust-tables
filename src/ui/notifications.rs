use std::time::SystemTime;

use ratatui::{
    layout::{Alignment, Rect},
    widgets::Paragraph,
};

pub enum NotifLevel {
    Debug,
    Warning,
    Recoverable,
    Fatal,
}

enum NotifListFocus {
    UnfocusedCollapsed,
    UnfocusedDisplaying,
    Focused,
}

pub struct Notif {
    created: SystemTime,
    content: String,
    level: NotifLevel,
}

pub struct NotifListState {
    notifs: Vec<Notif>,

    // Debug, notifications, warnings, errors
    counts: (u32, u32, u32, u32),

    // Indicates style of component to render (collapsed summary, display for recent
    // notifs, full scrollable browser)
    focus: NotifListFocus,
}

impl NotifListState {
    fn new(self) -> Self {
        NotifListState {
            notifs: vec![],
            counts: (0, 0, 0, 0),
            focus: NotifListFocus::UnfocusedCollapsed,
        }
    }

    // Push a notification on the state level.
    fn notify_user(mut self, level: NotifLevel, content: String) {
        match level {
            NotifLevel::Debug => self.counts.0 += 1,
            NotifLevel::Warning => self.counts.1 += 1,
            NotifLevel::Recoverable => self.counts.2 += 1,
            NotifLevel::Fatal => self.counts.3 += 1,
        }

        self.notifs.push(Notif {
            created: SystemTime::now(),
            content,
            level,
        })
    }
}

// Number of most recent notifs shown by the unfocused-displaying variant.
const MAX_DISPLAYED_NOTIFS: usize = 4;

// Lines available to render a single notif's content in the unfocused modes
// (and in focused mode without wrapping) before it's hard-truncated with an
// abrupt ellipsis.
const NOTIF_CONTENT_LINES: usize = 2;

// Width of the bottom-right panel used by the unfocused-displaying variant.
const NOTIF_LIST_WIDTH: u16 = 44;

fn notif_level_label(level: &NotifLevel) -> &'static str {
    match level {
        NotifLevel::Debug => "DEBUG",
        NotifLevel::Warning => "WARN",
        NotifLevel::Recoverable => "ERROR",
        NotifLevel::Fatal => "FATAL",
    }
}

fn format_counts(counts: &(u32, u32, u32, u32)) -> String {
    let (debug, warning, recoverable, unrecoverable) = counts;
    format!("D:{debug} W:{warning} E:{recoverable} F:{unrecoverable}")
}

// Hard-wraps `content` into at most `max_lines` lines of `width` characters,
// with no word-boundary awareness. If content remains after the last
// available line, that line is truncated further and suffixed with an
// abrupt ellipsis to signal loss.
fn truncate_content(content: &str, width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }

    let chars: Vec<char> = content.chars().collect();
    let mut lines = Vec::new();
    let mut idx = 0;
    while idx < chars.len() && lines.len() < max_lines {
        let end = (idx + width).min(chars.len());
        lines.push(chars[idx..end].iter().collect::<String>());
        idx = end;
    }

    if idx < chars.len() {
        if let Some(last) = lines.last_mut() {
            let keep = width.saturating_sub(1);
            let truncated: String = last.chars().take(keep).collect();
            *last = format!("{truncated}\u{2026}");
        }
    }

    lines
}

// Positions a `width` x `height` rect in the bottom-right corner of
// `frame_area`, clamping to the available space.
fn bottom_right_rect(frame_area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(frame_area.width);
    let height = height.min(frame_area.height);
    Rect {
        x: frame_area.right().saturating_sub(width),
        y: frame_area.bottom().saturating_sub(height),
        width,
        height,
    }
}

// Four-number summary of debug, warning, recoverable-error, and
// unrecoverable-error notif counts, right-aligned.
fn default_unfocused_collapsed_notif_list(state: &NotifListState) -> (Paragraph<'static>, Rect) {
    let text = format_counts(&state.counts);
    let width = text.chars().count() as u16;
    (
        Paragraph::new(text).alignment(Alignment::Right),
        Rect::new(0, 0, width, 1),
    )
}

// Borderless list of the most recent notifs (newest first), capped at
// MAX_DISPLAYED_NOTIFS with an upward-arrow indicator in the heading when
// more exist, followed by the count bar.
fn default_unfocused_displaying_notif_list(state: &NotifListState) -> (Paragraph<'static>, Rect) {
    let has_more = state.notifs.len() > MAX_DISPLAYED_NOTIFS;
    let heading = if has_more {
        "Notifications \u{2191}"
    } else {
        "Notifications"
    };

    let mut lines: Vec<String> = vec![heading.to_string()];

    for notif in state.notifs.iter().rev().take(MAX_DISPLAYED_NOTIFS) {
        let prefix = format!("[{}] ", notif_level_label(&notif.level));
        let content_width = (NOTIF_LIST_WIDTH as usize)
            .saturating_sub(prefix.len())
            .max(1);
        let wrapped = truncate_content(&notif.content, content_width, NOTIF_CONTENT_LINES);

        for (i, line) in wrapped.into_iter().enumerate() {
            if i == 0 {
                lines.push(format!("{prefix}{line}"));
            } else {
                lines.push(format!("{:width$}{line}", "", width = prefix.len()));
            }
        }
    }

    lines.push(String::new());
    lines.push(format_counts(&state.counts));

    let height = lines.len() as u16;
    (
        Paragraph::new(lines.join("\n")),
        Rect::new(0, 0, NOTIF_LIST_WIDTH, height),
    )
}

fn default_focused_notif_list(_state: &NotifListState) -> (Paragraph<'static>, Rect) {
    // Full scrollable browser: needs its own interaction model (scrolling,
    // selection, wrapping toggle) before it can be designed properly.
    todo!("design the focused notif browser")
}

// Conditional on NotifListFocus. Variants:
// - Collapsed, which is a four-number summary: the numbers of debug, warning, and error
// notifications displayed.
// - Unfocused, which is just a borderless list with a height limited to 4 messages and an
// indicator (upward arrow in the heading) when there's more than the limit. Set a constant for
// the maximum number of notifications displayed. The count bar appears below this.
// - Focused, which feeds into a todo! because I have to think about this state right now.
//
// In the unfocused modes and in focused mode without wrapping, the total number of lines
// available to a single notif is NOTIF_CONTENT_LINES with abrupt ellipsis truncation.
//
// Returns the rendered component along with its rect, positioned in the bottom-right corner of
// `frame_area`, for the caller's draw function to render into place.
fn default_notif_list(frame_area: Rect, notifs: &NotifListState) -> (Paragraph<'static>, Rect) {
    let (widget, size) = match notifs.focus {
        NotifListFocus::UnfocusedCollapsed => default_unfocused_collapsed_notif_list(notifs),
        NotifListFocus::UnfocusedDisplaying => default_unfocused_displaying_notif_list(notifs),
        NotifListFocus::Focused => default_focused_notif_list(notifs),
    };

    let area = bottom_right_rect(frame_area, size.width, size.height);
    (widget, area)
}

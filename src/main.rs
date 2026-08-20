mod repository;
mod tables;

use std::process::ExitCode;

use anyhow::Context;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    widgets::{Block, List, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use sqlx::{Pool, Sqlite};

use crate::tables::{TableId, TableSchema};

enum AppPage {
    Home,
    Table(TableId),
}

struct AppState {
    running: bool,
    page: AppPage,
}

fn render_home_page(
    app_state: &mut AppState,
    _pool: &Pool<Sqlite>,
    terminal: &mut DefaultTerminal,
) -> anyhow::Result<()> {
    // TODO: retrieve available tables and indicate loading state via pool

    // Determine the centered area for the table list's enclosing border,
    // capping the inner content height at half the screen.
    fn layout_list_area(frame_area: Rect, names: &[&str]) -> Rect {
        let max_list_height = frame_area.height.saturating_sub(2) / 2;
        let content_height = names.len() as u16;
        let list_height = content_height.min(max_list_height).max(1);
        let needs_scrollbar = content_height > max_list_height;

        let text_width = names.iter().map(|name| name.len()).max().unwrap_or(0) as u16 + 4;
        let width = (text_width + u16::from(needs_scrollbar) + 2).min(frame_area.width);

        let [vertical_area] = Layout::vertical([Constraint::Length(list_height + 2)])
            .flex(Flex::Center)
            .areas(frame_area);
        let [centered_area] = Layout::horizontal([Constraint::Length(width)])
            .flex(Flex::Center)
            .areas(vertical_area);

        centered_area
    }

    // Render the table list inside a bordered block, splitting off a
    // scrollbar column when the list doesn't fit its allotted area.
    fn draw_list(frame: &mut Frame, area: Rect, names: Vec<&str>, state: &mut ListState) {
        let content_height = names.len() as u16;
        let list = List::new(names).highlight_symbol("> ");

        let block = Block::bordered();
        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        if inner_area.height < content_height {
            let [list_area, scrollbar_area] =
                Layout::horizontal([Constraint::Min(0), Constraint::Length(1)]).areas(inner_area);

            frame.render_stateful_widget(list, list_area, state);

            let mut scrollbar_state = ScrollbarState::new(content_height as usize)
                .position(state.selected().unwrap_or(0));
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                scrollbar_area,
                &mut scrollbar_state,
            );
        } else {
            frame.render_stateful_widget(list, inner_area, state);
        }
    }

    // Compose layout and rendering into a single draw call.
    fn draw_home_page(frame: &mut Frame, names: Vec<&str>, state: &mut ListState) {
        let list_area = layout_list_area(frame.area(), &names);
        draw_list(frame, list_area, names, state);
    }

    let items = [
        TableSchema {
            id: TableId::new(),
            name: "table1".into(),
            columns: vec![],
        },
        TableSchema {
            id: TableId::new(),
            name: "table2".into(),
            columns: vec![],
        },
        TableSchema {
            id: TableId::new(),
            name: "table3".into(),
            columns: vec![],
        },
    ];
    terminal.clear().context("clearing terminal")?;
    let mut state = ListState::default().with_selected(Some(0));

    loop {
        match app_state.page {
            AppPage::Home => {
                // Draw current terminal state
                terminal
                    .draw(|frame| {
                        let names: Vec<&str> =
                            items.iter().map(|table| table.name.as_str()).collect();
                        draw_home_page(frame, names, &mut state);
                    })
                    .context("drawing table list")?;

                // Scan for terminal input and update list selector
                if let Event::Key(key) = event::read().context("reading terminal input")? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('k') | KeyCode::Up => {
                            state.select_previous();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            state.select_next();
                        }
                        // KeyCode::Char('h') | KeyCode::Left => continue,
                        // KeyCode::Char('l') | KeyCode::Right => continue,
                        KeyCode::Enter => {
                            if let Some(index) = state.selected() {
                                app_state.page = AppPage::Table(items[index].id.clone());
                                return Ok(());
                            }
                        }
                        KeyCode::Esc => {
                            app_state.running = false;
                            return Ok(());
                        }
                        _ => continue,
                    }
                }
            }
            _ => break,
        }
    }

    Ok(())
}

fn render_table_page(
    app_state: &mut AppState,
    _pool: &Pool<Sqlite>,
    terminal: &mut DefaultTerminal,
) -> anyhow::Result<()> {
    // TODO: retrieve table data via pool

    fn draw_table_page(frame: &mut Frame, table_id: &TableId, last_key: &Option<String>) {
        let text = format!("table page {}", table_id.0);
        let key_text = last_key.clone().unwrap_or_default();

        let [text_area, key_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
                .flex(Flex::Center)
                .areas(frame.area());

        frame.render_widget(Paragraph::new(text).alignment(Alignment::Center), text_area);
        frame.render_widget(
            Paragraph::new(key_text).alignment(Alignment::Center),
            key_area,
        );
    }
    let table_id = match &app_state.page {
        AppPage::Table(table_id) => table_id.clone(),
        _ => anyhow::bail!("unexpected page state while rendering table page"),
    };
    terminal.clear().context("clearing terminal")?;
    let mut last_key: Option<String> = None;

    loop {
        terminal
            .draw(|frame| draw_table_page(frame, &table_id, &last_key))
            .context("drawing table page")?;

        if let Event::Key(key) = event::read().context("reading terminal input")? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                // Update table view coordinates according to directions
                KeyCode::Char(c @ ('h' | 'j' | 'k' | 'l')) => {
                    last_key = Some(c.to_string());
                }
                // Exit to home page (table selector) on esc
                KeyCode::Esc => {
                    app_state.page = AppPage::Home;
                    return Ok(());
                }
                _ => {
                    last_key = Some("unknown key".into());
                }
            }
        }
    }
}

/// Primary application, evolving valid initial user data according to terminal input.
fn app(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let mut app_state = AppState {
        running: true,
        page: AppPage::Home,
    };

    while app_state.running {
        match &app_state.page {
            AppPage::Home => render_home_page(&mut app_state, pool, &mut terminal)
                .context("rendering home page")?,
            AppPage::Table(_) => render_table_page(&mut app_state, pool, &mut terminal)
                .context("rendering table page")?,
        }
    }

    ratatui::restore(); // Return user's terminal to original state
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    // Verify initial user data state
    let db = match repository::init_user_data().await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };

    // Application acts on valid user data
    match app(&db) {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

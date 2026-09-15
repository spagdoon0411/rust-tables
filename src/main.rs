mod database;
mod ui;

use anyhow::Context;
use clap::Parser;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::path::PathBuf;
use std::time::Duration;

use crate::database::Database;
use crate::ui::{Component, TableBrowser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: PathBuf,
}

fn poll_exit(running: &mut bool) -> anyhow::Result<()> {
    if event::poll(Duration::from_millis(250)).context("polling events")?
        && let Event::Key(key) = event::read().context("reading crossterm event")?
        && key.kind == KeyEventKind::Press
        && key.code == KeyCode::Char('c')
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        *running = false;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _ = Database::try_new(&args.path).await?;
    let mut terminal = ratatui::init();

    // Component tree defining the application
    let mut app = TableBrowser::new((1..32).map(|i| format!("Table {}", i)).collect());

    let mut running = true;
    while running {
        terminal.draw(|frame| {
            app.render(frame, frame.area());
        })?;

        poll_exit(&mut running)?;
    }

    ratatui::restore();

    Ok(())
}

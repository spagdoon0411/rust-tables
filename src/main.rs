mod database;
mod events;
mod ui;

use clap::Parser;
use crossterm::event::EventStream;
use std::path::PathBuf;

use crate::database::Database;
use crate::events::{AppEvent, collect_crossterm_event};
use crate::ui::{Component, TableBrowser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let _ = Database::try_new(&args.path).await?;
    let mut terminal = ratatui::init();
    let mut events = EventStream::new();

    // Component tree defining the application
    let mut app = TableBrowser::new((1..32).map(|i| format!("Table {}", i)).collect());

    let mut running = true;
    while running {
        terminal.draw(|frame| {
            app.render(frame, frame.area());
        })?;

        tokio::select! {
            event = collect_crossterm_event(&mut events) => {
                match event? {
                    AppEvent::Exit => running = false,
                    event => app.propagate_event(event),
                }
            }
        }
    }

    ratatui::restore();

    Ok(())
}

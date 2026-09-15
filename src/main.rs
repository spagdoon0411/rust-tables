mod database;
mod ui;

use clap::Parser;
use std::path::PathBuf;

use crate::database::Database;
use crate::ui::events::AppEvent;
use crate::ui::{Component, RatatuiUI, TableBrowser};

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
    let mut ui = RatatuiUI::new();

    // Component tree defining the application
    let mut app = TableBrowser::new((1..32).map(|i| format!("Table {}", i)).collect());

    let mut running = true;
    while running {
        ui.project(&mut app)?;

        tokio::select! {
            event = ui.next_event() => {
                match event? {
                    AppEvent::Exit => running = false,
                    event => app.propagate_event(event),
                }
            }
        }
    }

    Ok(())
}

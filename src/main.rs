mod clients;
mod database;
mod dispatcher;
mod store;
mod ui;

use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::clients::{AppAPIClient, SQLiteAPIClient};
use crate::dispatcher::Dispatcher;
use crate::store::SharedStore;
use crate::ui::{Component, RatatuiUI, SharedContext, TableBrowser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut ui = RatatuiUI::new();
    let mut dispatcher = Dispatcher::new();
    let mut client = SQLiteAPIClient::try_new(&args.path, dispatcher.action_tx.clone()).await?;
    let store = SharedStore::new();
    let context = SharedContext::new(
        store.clone(),
        client.request_tx.clone(),
        dispatcher.action_tx.clone(),
    );

    // Spawn and join dispatch and UI tasks:

    let shutdown = CancellationToken::new();

    // Dispatch task
    let dispatch_shutdown = shutdown.clone();
    let dispatch_task = tokio::spawn(async move {
        while !dispatch_shutdown.is_cancelled() {
            tokio::select! {
                request = client.next_request() => {
                    client.handle_request(request);
                }
                action = dispatcher.next_action() => {
                    store.handle_action(action);
                }
                _ = dispatch_shutdown.cancelled() => {}
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    // UI task
    let ui_shutdown = shutdown.clone();
    let ui_task = tokio::spawn(async move {
        let mut app = TableBrowser::new(Arc::clone(&context));

        let on_exit = || ui_shutdown.cancel();
        while !ui_shutdown.is_cancelled() {
            let event = ui.next_event(on_exit).await?;
            app.propagate_event(event);
            ui.project(&mut app)?;
        }

        Ok::<(), anyhow::Error>(())
    });

    let (dispatch_result, ui_result) = tokio::join!(dispatch_task, ui_task);
    dispatch_result??;
    ui_result??;

    Ok(())
}

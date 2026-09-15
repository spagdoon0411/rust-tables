mod database;

use clap::Parser;
use std::path::PathBuf;

use crate::database::Database;

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

    Ok(())
}

mod tables;

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

const USER_DATA_DIR: &str = "user_data";

fn verify_user_data() -> io::Result<()> {
    if Path::new(USER_DATA_DIR).is_dir() {
        println!("Found local data.")
    } else {
        println!("No user data found.");

        print!("Create user_data directory? (y/n): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("y") {
            fs::create_dir(USER_DATA_DIR)?;
            println!("Created user_data directory.");
        } else {
            println!("Skipped creating user_data directory.");
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    match verify_user_data() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    }

    let col_schema = tables::ColumnSchema::new("col1", tables::ColumnType::String);
    let tab_schema = tables::TableSchema::new("tab1", vec![col_schema]);

    // TODO: enter primary application

    ExitCode::SUCCESS
}

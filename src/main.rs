use anyhow::{bail, Result};
use codecrafters_sqlite::Sqlite;

fn main() -> Result<()> {
    // Parse arguments
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => bail!("Missing <database path> and <command>"),
        2 => bail!("Missing <command>"),
        _ => {}
    }

    // Parse command and act accordingly
    let command = &args[2];
    match command.as_str() {
        ".dbinfo" => {
            let sqlite = Sqlite::open_for_read(&args[1])?;

            // You can use print statements as follows for debugging, they'll be visible when running tests.
            // eprintln!("Logs from your program will appear here!");

            println!("database page size: {}", sqlite.header.page_size);
            println!("number of tables: {}", sqlite.schema_page.table_count())
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}

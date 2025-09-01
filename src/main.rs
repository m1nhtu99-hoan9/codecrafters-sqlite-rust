use anyhow::{bail, Result};
use codecrafters_sqlite::{Sqlite, sql::SqlQuery};

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
        },
        ".tables" => {
            let sqlite = Sqlite::open_for_read(&args[1])?;

            println!("database page size: {}", sqlite.header.page_size);
            println!("table names: ");
            for tbl_name in sqlite.schema_page.table_names()?.iter() {
                print!("{} ", tbl_name);
            }
        },
        query if query.to_uppercase().starts_with("SELECT") => {
            let mut sqlite = Sqlite::open_for_read(&args[1])?;
            let parsed_query = SqlQuery::parse(query)?;
            let result = sqlite.execute_query(&parsed_query)?;
            println!("{}", result);
        },
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}

//! SQLite Database File Structure and B-tree Organisation
//!
//! ```text
//! ┌── SQLite Database File (.db) ──────────────────────────────────────────────┐
//! │                                                                            │
//! │ ┌── Page 1 (4096 bytes) ───────────────────────────────────────────────┐   │
//! │ │ DB Header  │ sqlite_schema B-tree (system catalogue)                │    │
//! │ │ (100 bytes)                                                          │   │
//! │ │            │ Page Header: type=0x0d, cells=3, pointers=[0x0ec3]     │    │
//! │ │            │                                                         │   │
//! │ │            │ Cell at 0x0ec3: ┌── Record Header ──────────────────┐ │     │
//! │ │            │                 │ header_size: 07                  │ │      │
//! │ │            │                 │ column_types: [17,1b,1b,01]      │ │      │
//! │ │            │                 │   ↓                              │ │      │
//! │ │            │                 │ TEXT(2) TEXT(7) TEXT(7) INT      │ │      │
//! │ │            │                 └──────────────────────────────────┘ │      │
//! │ │            │                 ┌── Data ───────────────────────────┐ │     │
//! │ │            │                 │ "table" "oranges" "oranges"       │ │     │
//! │ │            │                 └───────────────────────────────────┘ │     │
//! │ └───────────────────────────────────────────────────────────────────────┘  │
//! │                                                                            │
//! │ ┌── Page 2 (4096 bytes) ───────────────────────────────────────────────┐   │
//! │ │ "apples" table B-tree root                                          │    │
//! │ │                                                                     │    │
//! │ │ Page Header: type=0x0d, cells=2, pointers=[0x0f80, 0x0f40]         │     │
//! │ │                                                                     │    │
//! │ │ Cell at 0x0f80: ┌── Record Header ───────────┐                     │     │
//! │ │                 │ column_types: [01, 19]     │ ← ColumnType HERE   │     │
//! │ │                 │   ↓                        │                     │     │
//! │ │                 │ INT(1)  TEXT(3)            │                     │     │
//! │ │                 └────────────────────────────┘                     │     │
//! │ │                 ┌── Data ─────────────────────┐                     │    │
//! │ │                 │ 1 "red"                     │                     │    │
//! │ │                 └─────────────────────────────┘                     │    │
//! │ │                                                                     │    │
//! │ │ Cell at 0x0f40: ┌── Record Header ───────────┐                     │     │
//! │ │                 │ column_types: [01, 1d]     │ ← Different layout! │     │
//! │ │                 │   ↓                        │                     │     │
//! │ │                 │ INT(1)  TEXT(5)            │                     │     │
//! │ │                 └────────────────────────────┘                     │     │
//! │ │                 ┌── Data ─────────────────────┐                     │    │
//! │ │                 │ 2 "green"                   │                     │    │
//! │ │                 └─────────────────────────────┘                     │    │
//! │ └───────────────────────────────────────────────────────────────────────┘  │
//! │                                                                            │
//! │ ┌── Page 3 (4096 bytes) ───────────────────────────────────────────────┐   │
//! │ │ "oranges" table B-tree root                                         │    │
//! │ │ (completely separate table data)                                    │    │
//! │ └───────────────────────────────────────────────────────────────────────┘  │
//! └────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Architecture Insights
//!
//! - **Each PAGE = One table's B-tree node** (not multiple tables per page)
//! - **Each CELL = One table record** with its own ColumnType metadata  
//! - **ColumnType lives in RECORD HEADERS**, not page headers
//! - **Different records can have different column layouts** (same schema, different storage)
//! - **Page header** contains: page type, cell count, cell pointers
//! - **Cell content** contains: payload size, row ID, record header, actual data
//!
//! The critical insight: `ColumnType` serial codes (17, 1b, 01, etc.) are embedded
//! **inside each individual record**, allowing SQLite to optimise storage per-row
//! while maintaining schema flexibility.

use anyhow::{bail, Result};
use codecrafters_sqlite::{sql, Sqlite, query::QueryExecutor};

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
        ".tables" => {
            let sqlite = Sqlite::open_for_read(&args[1])?;

            println!("database page size: {}", sqlite.header.page_size);
            println!("table names: ");
            for tbl_name in sqlite.schema_page.table_names()?.iter() {
                print!("{} ", tbl_name);
            }
        }
        query if query.to_uppercase().starts_with("SELECT") => {
            let mut sqlite = Sqlite::open_for_read(&args[1])?;
            let sql_stmt = sql::parse_sql(query)?;
            
            // Use the new query executor for all SELECT statements
            let executor = QueryExecutor;
            let result = executor.execute(&mut sqlite, sql_stmt)?;
            
            // Display results
            for row in result.rows {
                let output = row.values.join(" ");
                println!("{}", output);
            }
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}

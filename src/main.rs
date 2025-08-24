use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::path::Path;
use std::fs;

mod storage;
use storage::Page;

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
            let path = Path::new(&args[1]);
            let mut file = open_sqlite_file(path)?;
            let header = read_sqlite_file_header(&mut file, path)?;

            // The page size is stored at the 16th byte offset, using 2 bytes in big-endian order
            let page_size = u16::from_be_bytes([header[16], header[17]]);
            
            let master_page = Page::new(&mut file, path)?;

            // You can use print statements as follows for debugging, they'll be visible when running tests.
            eprintln!("Logs from your program will appear here!");

            println!("database page size: {}", page_size);
            println!("number of tables: {}", master_page.cell_count()?)
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}

/// Open the file with exhaustive handling (for educational purpose)
fn open_sqlite_file(path: &Path) -> Result<File> {
    let abs_path = fs::canonicalize(path)
        .with_context(|| format!("SQL file not found {}", path.display()))?;
    let path_str = abs_path.display();

    match File::open(&abs_path) {
        Ok(f) => Ok(f),
        Err(e) => {
            match e.kind() {
                ErrorKind::PermissionDenied => bail!("Permission denied opening SQLite file: {}", path_str),
                ErrorKind::InvalidInput => bail!("Invalid path for SQLite file: {}", path_str),
                ErrorKind::AlreadyExists => bail!("File already exists but cannot be opened (conflict): {}", path_str), // Rare for open
                ErrorKind::AddrInUse | ErrorKind::AddrNotAvailable => bail!("Address-related error opening SQLite file (possible network file): {}", path_str),
                ErrorKind::BrokenPipe => bail!("Broken pipe error opening SQLite file: {}", path_str),
                ErrorKind::ConnectionAborted => bail!("Connection aborted while opening SQLite file: {}", path_str),
                ErrorKind::ConnectionRefused => bail!("Connection refused for SQLite file: {}", path_str),
                ErrorKind::ConnectionReset => bail!("Connection reset while opening SQLite file: {}", path_str),
                ErrorKind::Interrupted => bail!("Operation interrupted while opening SQLite file: {}", path_str),
                ErrorKind::InvalidData => bail!("Invalid data encountered opening SQLite file: {}", path_str),
                ErrorKind::NotConnected => bail!("Not connected error opening SQLite file: {}", path_str),
                ErrorKind::TimedOut => bail!("Timeout opening SQLite file: {}", path_str),
                ErrorKind::UnexpectedEof => bail!("Unexpected EOF while opening SQLite file: {}", path_str), // Rare for open
                ErrorKind::WouldBlock => bail!("Operation would block while opening SQLite file: {}", path_str),
                ErrorKind::WriteZero => bail!("Write zero error (unexpected for open) on SQLite file: {}", path_str),
                ErrorKind::Other => Err(e).with_context(|| format!("Unknown error opening SQLite file: {}", path_str)),
                _ => Err(e).with_context(|| format!("Unexpected error opening SQLite file: {}", path_str)),
            }
        }
    }
}

fn read_sqlite_file_header(file: &mut File, path: &Path) -> Result<[u8; 100]> {
    let mut header = [0; 100];
    let file_name = path.file_name().unwrap().to_str().unwrap();
    match file.read_exact(&mut header) {
        Ok(()) => {
            // Optional: Validate magic header for exhaustiveness
            let magic = &header[0..16];
            if magic != b"SQLite format 3\0" {
                bail!("Invalid SQLite header magic in file '{}': expected 'SQLite format 3\\0', got {:?}", file_name, magic);
            }
            Ok(header)
        }
        Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => bail!("SQLite file '{}' is too small (less than 100 bytes for header)", file_name),
            ErrorKind::PermissionDenied => bail!("Permission denied reading header from SQLite file: {}", file_name),
            ErrorKind::Interrupted => bail!("Read interrupted while fetching SQLite header from: {} (consider retrying)", file_name),
            ErrorKind::BrokenPipe => bail!("Broken pipe during header read from SQLite file: {}", file_name),
            ErrorKind::ConnectionAborted => bail!("Connection aborted reading SQLite header: {}", file_name),
            ErrorKind::ConnectionRefused => bail!("Connection refused reading SQLite header: {}", file_name),
            ErrorKind::ConnectionReset => bail!("Connection reset reading SQLite header: {}", file_name),
            ErrorKind::InvalidData => bail!("Invalid data encountered reading SQLite header: {}", file_name),
            ErrorKind::NotConnected => bail!("Not connected error reading SQLite header: {}", file_name),
            ErrorKind::TimedOut => bail!("Timeout reading SQLite header from: {}", file_name),
            ErrorKind::WouldBlock => bail!("Operation would block reading SQLite header: {}", file_name),
            ErrorKind::WriteZero => bail!("Write zero error (unexpected for read) on SQLite header: {}", file_name),
            ErrorKind::Other => Err(e).with_context(|| format!("Unknown error reading SQLite header from: {}", file_name)),
            _ => Err(e).with_context(|| format!("Unexpected error reading SQLite header from: {}", file_name)),
        }
    }
}


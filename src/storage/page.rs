use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::Path;

const DATABASE_HEADER_SIZE: usize = 100;

/// SQLite database header (first 100 bytes)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DatabaseHeader {
    pub page_size: u16,
    header_data: [u8; DATABASE_HEADER_SIZE],
}

impl DatabaseHeader {
    pub fn read_from_file(file: &mut File, path: &Path) -> Result<Self> {
        let mut header = [0; DATABASE_HEADER_SIZE];
        let file_name = path.file_name().unwrap().to_str().unwrap();

        match file.read_exact(&mut header) {
            Ok(()) => {
                // Validate magic header
                let magic = &header[0..16];
                if magic != b"SQLite format 3\0" {
                    bail!(
                        "Invalid SQLite header magic in file '{}': expected 'SQLite format 3\\0', got {:?}",
                        file_name,
                        magic
                    );
                }

                // Extract page size from bytes 16-17
                let page_size = u16::from_be_bytes([header[16], header[17]]);

                Ok(Self {
                    page_size,
                    header_data: header,
                })
            }
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => bail!(
                    "SQLite file '{}' is too small (less than 100 bytes for header)",
                    file_name
                ),
                ErrorKind::PermissionDenied => {
                    bail!("Permission denied reading header from SQLite file: {}", file_name)
                }
                ErrorKind::Interrupted => bail!(
                    "Read interrupted while fetching SQLite header from: {} (consider retrying)",
                    file_name
                ),
                ErrorKind::BrokenPipe => bail!("Broken pipe during header read from SQLite file: {}", file_name),
                ErrorKind::ConnectionAborted => {
                    bail!("Connection aborted reading SQLite header: {}", file_name)
                }
                ErrorKind::ConnectionRefused => {
                    bail!("Connection refused reading SQLite header: {}", file_name)
                }
                ErrorKind::ConnectionReset => {
                    bail!("Connection reset reading SQLite header: {}", file_name)
                }
                ErrorKind::InvalidData => bail!("Invalid data encountered reading SQLite header: {}", file_name),
                ErrorKind::NotConnected => {
                    bail!("Not connected error reading SQLite header: {}", file_name)
                }
                ErrorKind::TimedOut => bail!("Timeout reading SQLite header from: {}", file_name),
                ErrorKind::WouldBlock => {
                    bail!("Operation would block reading SQLite header: {}", file_name)
                }
                ErrorKind::WriteZero => bail!("Write zero error (unexpected for read) on SQLite header: {}", file_name),
                ErrorKind::Other => {
                    Err(e).with_context(|| format!("Unknown error reading SQLite header from: {}", file_name))
                }
                _ => Err(e).with_context(|| format!("Unexpected error reading SQLite header from: {}", file_name)),
            },
        }
    }
}

/// Raw page container with dynamic size based on database header
///
/// Design choice: Vec<u8> for variable page sizes, validated against
/// the actual page size from database header. Future: migrate to
/// `memmap2` for disk-based volatile storage.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    data: Vec<u8>,
    page_size: u16,
}

impl Page {
    pub fn new(file: &mut File, path: &Path, header: &DatabaseHeader) -> Result<Self> {
        let mut data = vec![0; header.page_size as usize];
        let file_name = path.file_name().unwrap().to_str().unwrap();

        match file.read_exact(&mut data) {
            Ok(()) => Ok(Self {
                data,
                page_size: header.page_size,
            }),
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => bail!(
                    "Page file '{}' is too small (less than {} bytes)",
                    file_name,
                    header.page_size
                ),
                ErrorKind::PermissionDenied => {
                    bail!("Permission denied reading page from file: {}", file_name)
                }
                _ => Err(e).with_context(|| format!("Unexpected error reading page from: {}", file_name)),
            },
        }
    }

    pub fn cell_count(&self) -> Result<u16> {
        Ok(u16::from_be_bytes([self.data[3], self.data[4]]))
    }
}

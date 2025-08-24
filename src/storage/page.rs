use std::io::{ErrorKind, Read};
use std::fs::File;
use std::path::Path;
use anyhow::{bail, Result, Context};

const PAGE_SIZE: usize = 4096;

/// Raw 4KB page container optimized for cache locality
/// 
/// Design choice: Box<[u8; PAGE_SIZE]> for cache locality over Vec<u8>
/// since we access by indexes/ranges regularly. Future: migrate to
/// `memmap2` for disk-based volatile storage with same interface.
pub struct Page {
    data: Box<[u8; PAGE_SIZE]>,
}

impl Page {
    pub fn new(file: &mut File, path: &Path) -> Result<Self> {
        let mut data = [0; PAGE_SIZE];
        let file_name = path.file_name().unwrap().to_str().unwrap();
        match file.read_exact(&mut data) {
            Ok(()) => {

                Ok(Self {
                    data: Box::new(data),
                })
            }
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => bail!("Page file '{}' is too small (less than {} bytes)", file_name, PAGE_SIZE),
                ErrorKind::PermissionDenied => bail!("Permission denied reading page from file: {}", file_name),
                ErrorKind::Interrupted => bail!("Read interrupted while loading page from: {} (consider retrying)", file_name),
                ErrorKind::BrokenPipe => bail!("Broken pipe during page read from file: {}", file_name),
                ErrorKind::ConnectionAborted => bail!("Connection aborted reading page: {}", file_name),
                ErrorKind::ConnectionRefused => bail!("Connection refused reading page: {}", file_name),
                ErrorKind::ConnectionReset => bail!("Connection reset reading page: {}", file_name),
                ErrorKind::InvalidData => bail!("Invalid data encountered reading page: {}", file_name),
                ErrorKind::NotConnected => bail!("Not connected error reading page: {}", file_name),
                ErrorKind::TimedOut => bail!("Timeout reading page from: {}", file_name),
                ErrorKind::WouldBlock => bail!("Operation would block reading page: {}", file_name),
                ErrorKind::WriteZero => bail!("Write zero error (unexpected for read) on page: {}", file_name),
                ErrorKind::Other => Err(e).with_context(|| format!("Unknown error reading page from: {}", file_name)),
                _ => Err(e).with_context(|| format!("Unexpected error reading page from: {}", file_name)),
            }
        }
    }

    pub fn cell_count(&self) -> Result<u16> {
        if self.data.len() < 4 {
            bail!("Page too small to contain cell count");
        }
        Ok(u16::from_be_bytes([self.data[3], self.data[4]]))
    }
}
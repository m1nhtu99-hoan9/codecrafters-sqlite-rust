use anyhow::{bail, Context};
use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf}
};
use crate::{
    PageZero,
    paging::Pager,
    storage::page::DbHeader
};

pub struct Sqlite<F> {
    pub pager: Pager<F>,
    pub file_path: PathBuf,
    pub header: DbHeader,
    pub schema_page: PageZero
}

impl <F> Sqlite<F> {
   pub fn file_name(&self) -> String {
       self.file_path.file_name().unwrap().to_str().unwrap().to_string()
   }
}


impl Sqlite<File> {
    /// Open the file with exhaustive handling (for educational purpose)
    pub fn open_for_read(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path_ptr: &Path = path.as_ref();
        let abs_path =
            path_ptr.canonicalize().with_context(|| format!("SQL file not found {}", path_ptr.display()))?;
        let path_str = abs_path.display();

        let file = match File::options().read(true).write(false).open(&abs_path) {
            Ok(f) => Ok(f),
            Err(e) => {
                match e.kind() {
                    ErrorKind::PermissionDenied => bail!("Permission denied opening SQLite file: {}", path_str),
                    ErrorKind::InvalidInput => bail!("Invalid path for SQLite file: {}", path_str),
                    ErrorKind::AlreadyExists => {
                        bail!("File already exists but cannot be opened (conflict): {}", path_str)
                    } // Rare for open
                    ErrorKind::AddrInUse | ErrorKind::AddrNotAvailable => bail!(
                        "Address-related error opening SQLite file (possible network file): {}",
                        path_str
                    ),
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
                    ErrorKind::WriteZero => {
                        bail!("Write zero error (unexpected for open) on SQLite file: {}", path_str)
                    }
                    ErrorKind::Other => {
                        Err(e).with_context(|| format!("Unknown error opening SQLite file: {}", path_str))
                    }
                    _ => Err(e).with_context(|| format!("Unexpected error opening SQLite file: {}", path_str)),
                }
            }
        };

        let mut file = file?;
        file.seek(SeekFrom::Start(0))?;
        
        let mut header_buf = [0; 100];
        let file_name = abs_path.file_name().unwrap().to_str().unwrap();
        
        match file.read_exact(&mut header_buf) {
            Ok(()) => {
                let header = DbHeader::parse_from(header_buf, &abs_path)?;
                let mut pager = Pager::new(file, header.page_size as usize);
                
                // Skip the 100-byte header to read page zero
                pager.input.seek(SeekFrom::Start(100))?;
                
                let page_size = header.page_size as usize;
                let mut page_zero_data = vec![0; page_size];
                match pager.input.read_exact(&mut page_zero_data) {
                    Ok(()) => {
                        let page_zero = PageZero::init(page_zero_data)?;
                        
                        Ok(Self {
                            pager,
                            file_path: abs_path,
                            header,
                            schema_page: page_zero,
                        })
                    }
                    Err(e) => match e.kind() {
                        ErrorKind::UnexpectedEof => bail!(
                            "SQLite file '{}' page zero is too small (less than {} bytes)",
                            file_name,
                            page_size
                        ),
                        _ => Err(e).with_context(|| format!("Error reading page zero from: {}", file_name)),
                    }
                }
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
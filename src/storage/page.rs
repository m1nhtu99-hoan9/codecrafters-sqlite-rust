use crate::storage::btree::{BTreePageHeader, PageType};
use anyhow::{bail, Result};
use std::path::Path;

const DATABASE_HEADER_SIZE: usize = 100;

/// SQLite database header (first 100 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct DbHeader {
    pub page_size: u16,
    header_data: [u8; DATABASE_HEADER_SIZE],
}

impl DbHeader {
    pub fn parse_from(header: [u8; DATABASE_HEADER_SIZE], path: impl AsRef<Path>) -> Result<Self> {
        let file_name = path.as_ref().file_name().unwrap().to_str().unwrap();

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageZero {
    first_freeblock: u16,
    cell_count: u16,
    cell_content_start: u16,
    fragmented_bytes: u8,
    cell_pointers: Vec<u16>,
}

impl PageZero {
    pub fn init(buffer: Vec<u8>) -> Result<Self> {
        let header = BTreePageHeader::parse(&buffer)?;
        
        // Assert this is a leaf table page (sqlite_schema table must be leaf table)
        if !matches!(header.page_type, PageType::LeafTable) {
            bail!("Page zero must be a leaf table page (sqlite_schema), found {:?}", header.page_type);
        }

        // Assert no rightmost pointer for leaf pages
        if header.rightmost_pointer.is_some() {
            bail!("Page zero (leaf table) should not have rightmost pointer");
        }

        // Assert reasonable cell content start (should be > header size + cell pointer array)
        let min_content_start = 8 + (header.cell_count as usize * 2); // header + cell pointers
        if (header.cell_content_start as usize) < min_content_start {
            bail!("Invalid cell content start: {} < {}", header.cell_content_start, min_content_start);
        }

        Ok(Self {
            first_freeblock: header.first_freeblock,
            cell_count: header.cell_count,
            cell_content_start: header.cell_content_start,
            fragmented_bytes: header.fragmented_bytes,
            cell_pointers: header.cell_pointers,
        })
    }

    pub fn table_count(&self) -> u16 {
        self.cell_count
    }

    pub fn cells(&self) -> impl Iterator<Item = u16> + '_ {
        self.cell_pointers.iter().copied()
    }
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct LeafIndexPage {
    pub first_freeblock: u16,
    pub cell_count: u16,
    pub cell_content_start: u16,
    pub fragmented_bytes: u8,
    pub cell_pointers: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LeafTablePage {
    pub first_freeblock: u16,
    pub cell_count: u16,
    pub cell_content_start: u16,
    pub fragmented_bytes: u8,
    pub cell_pointers: Vec<u16>,
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct InteriorIndexPage {
    pub first_freeblock: u16,
    pub cell_count: u16,
    pub cell_content_start: u16,
    pub fragmented_bytes: u8,
    pub rightmost_pointer: u32,
    pub cell_pointers: Vec<u16>,
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct InteriorTablePage {
    pub first_freeblock: u16,
    pub cell_count: u16,
    pub cell_content_start: u16,
    pub fragmented_bytes: u8,
    pub rightmost_pointer: u32,
    pub cell_pointers: Vec<u16>,
}

impl LeafIndexPage {
    pub fn parse(_buffer: &[u8]) -> Result<Self> {
        todo!()
    }
}

impl LeafTablePage {
    pub fn parse(buffer: &[u8]) -> Result<Self> {
        let header = BTreePageHeader::parse(buffer)?;

        // Validate that this is actually a leaf table page
        if !matches!(header.page_type, PageType::LeafTable) {
            bail!("Expected leaf table page, found {:?}", header.page_type);
        }

        Ok(Self {
            first_freeblock: header.first_freeblock,
            cell_count: header.cell_count,
            cell_content_start: header.cell_content_start,
            fragmented_bytes: header.fragmented_bytes,
            cell_pointers: header.cell_pointers,
        })
    }
}

impl InteriorIndexPage {
    pub fn parse(_buffer: &[u8]) -> Result<Self> {
        todo!()
    }
}

impl InteriorTablePage {
    pub fn parse(_buffer: &[u8]) -> Result<Self> {
        todo!()
    }
}

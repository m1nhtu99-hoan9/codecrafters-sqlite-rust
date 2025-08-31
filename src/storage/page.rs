use crate::storage::btree::{BTreePageHeader, LeafTableCell, PageType, SchemaMasterRecord};
use anyhow::{bail, Result};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaPage {
    first_freeblock: u16,
    cell_count: u16,
    cell_content_start: u16,
    fragmented_bytes: u8,
    cell_pointers: Vec<u16>,
    buffer: Vec<u8>,
}

impl SchemaPage {
    pub fn init(buffer: Vec<u8>) -> Result<Self> {
        let header = BTreePageHeader::parse(&buffer)?;

        // Assert this is a leaf table page (sqlite_schema table must be leaf table)
        if !matches!(header.page_type, PageType::LeafTable) {
            bail!(
                "Page zero must be a leaf table page (sqlite_schema), found {:?}",
                header.page_type
            );
        }

        // Assert no rightmost pointer for leaf pages
        if header.rightmost_pointer.is_some() {
            bail!("Page zero (leaf table) should not have rightmost pointer");
        }

        // Assert reasonable cell content start (should be > header size + cell pointer array)
        let min_content_start = 8 + (header.cell_count as usize * 2); // header + cell pointers
        if (header.cell_content_start as usize) < min_content_start {
            bail!(
                "Invalid cell content start: {} < {}",
                header.cell_content_start,
                min_content_start
            );
        }

        Ok(Self {
            first_freeblock: header.first_freeblock,
            cell_count: header.cell_count,
            cell_content_start: header.cell_content_start,
            fragmented_bytes: header.fragmented_bytes,
            cell_pointers: header.cell_pointers,
            buffer,
        })
    }

    pub fn table_count(&self) -> u16 {
        self.cell_count
    }

    pub fn cells(&self) -> impl Iterator<Item = u16> + '_ {
        self.cell_pointers.iter().copied()
    }

    /// Extract table names from sqlite_schema
    pub fn table_names(&self) -> Result<Rc<[String]>> {
        let mut table_names = Vec::new();

        for cell_offset in self.cells() {
            // Cell offsets are relative to page start, but our buffer excludes the 100-byte DB header
            let adjusted_offset = (cell_offset as usize).saturating_sub(100);
            let cell = LeafTableCell::parse(&self.buffer, adjusted_offset as u16)?;
            let schema_record = SchemaMasterRecord::from_cell(&self.buffer, &cell)?;

            // Only collect actual tables (not indexes, views, etc.)
            if schema_record.type_ == "table" {
                table_names.push(schema_record.name);
            }
        }

        Ok(table_names.into())
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

use crate::storage::btree::{BTreePageHeader, LeafTableCell, PageType, SchemaMasterRecord};
use anyhow::{bail};
use std::rc::Rc;
use crate::DATABASE_HEADER_SIZE;

/// The `sqlite_schema` page
#[derive(Debug, Clone, PartialEq)]
pub struct RootPage {
    first_freeblock: u16,
    cell_count: u16,
    cell_content_start: u16,
    fragmented_bytes: u8,
    cell_pointers: Vec<u16>,
    buffer: Vec<u8>,
}

impl RootPage {
    pub fn init(buffer: Vec<u8>) -> anyhow::Result<Self> {
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

        // Adjust all cell pointers at construction time since our buffer excludes the 100-byte DB header
        let adjusted_cell_pointers: Vec<u16> = header.cell_pointers
            .into_iter()
            .map(|offset| offset.saturating_sub(DATABASE_HEADER_SIZE as u16))
            .collect();

        Ok(Self {
            first_freeblock: header.first_freeblock,
            cell_count: header.cell_count,
            cell_content_start: header.cell_content_start,
            fragmented_bytes: header.fragmented_bytes,
            cell_pointers: adjusted_cell_pointers,
            buffer,
        })
    }

    #[inline]
    pub fn table_count(&self) -> u16 {
        self.cell_count
    }

    #[inline]
    pub fn cells(&self) -> impl Iterator<Item = u16> + '_ {
        self.cell_pointers.iter().copied()
    }

    /// Extract table names from sqlite_schema
    pub fn table_names(&self) -> anyhow::Result<Rc<[String]>> {
        self.cells()
            .map(|cell_offset| -> anyhow::Result<Option<String>> {
                let cell = LeafTableCell::parse(&self.buffer, cell_offset)?;
                let schema_record = SchemaMasterRecord::from_cell(&self.buffer, &cell)?;
                if schema_record.type_ == "table" {
                    Ok(Some(schema_record.name))
                } else {
                    Ok(None)
                }
            })
            .filter_map(|result| result.transpose())
            .collect::<anyhow::Result<Vec<_>>>()
            .map(|names| names.into())
    }

    /// Find a specific table's schema record by name
    pub fn find_table(&self, table_name: &str) -> anyhow::Result<Option<SchemaMasterRecord>> {
        for cell_offset in self.cells() {
            let cell = LeafTableCell::parse(&self.buffer, cell_offset)?;
            let schema_record = SchemaMasterRecord::from_cell(&self.buffer, &cell)?;
            
            if schema_record.type_ == "table" && schema_record.name == table_name {
                return Ok(Some(schema_record));
            }
        }
        Ok(None)
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
    #[allow(unused)]
    pub fn parse(_buffer: &[u8]) -> anyhow::Result<Self> {
        todo!()
    }
}

impl LeafTablePage {
    pub fn parse(buffer: &[u8]) -> anyhow::Result<Self> {
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
    #[allow(unused)]
    pub fn parse(_buffer: &[u8]) -> anyhow::Result<Self> {
        todo!()
    }
}

impl InteriorTablePage {
    #[allow(unused)]
    pub fn parse(_buffer: &[u8]) -> anyhow::Result<Self> {
        todo!()
    }
}

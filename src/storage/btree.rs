use anyhow::bail;
use crate::storage::page::{InteriorIndexPage, InteriorTablePage, LeafIndexPage, LeafTablePage};

#[derive(Debug, Clone, PartialEq)]
pub enum PageType {
    LeafIndex = 0x0a,
    LeafTable = 0x0d,
    InteriorIndex = 0x02,
    InteriorTable = 0x05,
}

impl PageType {
    pub fn from_byte(byte: u8) -> anyhow::Result<Self> {
        match byte {
            0x0a => Ok(PageType::LeafIndex),
            0x0d => Ok(PageType::LeafTable),
            0x02 => Ok(PageType::InteriorIndex),
            0x05 => Ok(PageType::InteriorTable),
            other => bail!("Invalid page type: 0x{:02x}", other),
        }
    }
}


/// Common B-tree page header structure shared by all SQLite page types
#[derive(Debug, Clone, PartialEq)]
pub struct BTreePageHeader {
    pub page_type: PageType,
    pub first_freeblock: u16,
    pub cell_count: u16,
    pub cell_content_start: u16,
    pub fragmented_bytes: u8,
    pub rightmost_pointer: Option<u32>, // Only for interior pages
    pub cell_pointers: Vec<u16>,
}


impl BTreePageHeader {
    pub fn parse(buffer: &[u8]) -> anyhow::Result<Self> {
        if buffer.len() < 8 {
            bail!("Page buffer too small for B-tree page header");
        }

        let page_type = PageType::from_byte(buffer[0])?;

        let first_freeblock = u16::from_be_bytes([buffer[1], buffer[2]]);
        let cell_count = u16::from_be_bytes([buffer[3], buffer[4]]);
        let cell_content_start = u16::from_be_bytes([buffer[5], buffer[6]]);
        let fragmented_bytes = buffer[7];

        let (rightmost_pointer, header_size) = match page_type {
            PageType::InteriorIndex | PageType::InteriorTable => {
                if buffer.len() < 12 {
                    bail!("Interior page buffer too small for full header");
                }
                let pointer = u32::from_be_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]);
                (Some(pointer), 12)
            }
            _ => (None, 8),
        };

        // Extract cell pointers
        let mut cell_pointers = Vec::with_capacity(cell_count as usize);
        for i in 0..cell_count {
            let offset = header_size + (i as usize * 2);
            if offset + 1 < buffer.len() {
                let pointer = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
                cell_pointers.push(pointer);
            }
        }

        Ok(Self {
            page_type,
            first_freeblock,
            cell_count,
            cell_content_start,
            fragmented_bytes,
            rightmost_pointer,
            cell_pointers,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BTreePage {
    LeafIndex(LeafIndexPage),
    LeafTable(LeafTablePage),
    InteriorIndex(InteriorIndexPage),
    InteriorTable(InteriorTablePage),
}

impl BTreePage {
    pub fn parse(buffer: &[u8]) -> anyhow::Result<Self> {
        if buffer.len() < 8 {
            bail!("Page buffer too small for B-tree page header");
        }

        match PageType::from_byte(buffer[0])? {
            PageType::LeafIndex => Ok(Self::LeafIndex(LeafIndexPage::parse(buffer)?)),
            PageType::LeafTable => Ok(Self::LeafTable(LeafTablePage::parse(buffer)?)),
            PageType::InteriorIndex => Ok(Self::InteriorIndex(InteriorIndexPage::parse(buffer)?)),
            PageType::InteriorTable => Ok(Self::InteriorTable(InteriorTablePage::parse(buffer)?)),
        }
    }

    pub fn cell_count(&self) -> u16 {
        match self {
            Self::LeafIndex(p) => p.cell_count,
            Self::LeafTable(p) => p.cell_count,
            Self::InteriorIndex(p) => p.cell_count,
            Self::InteriorTable(p) => p.cell_count,
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::LeafIndex(_) | Self::LeafTable(_))
    }

    pub fn is_table_page(&self) -> bool {
        matches!(self, Self::LeafTable(_) | Self::InteriorTable(_))
    }

    pub fn cells(&self) -> impl Iterator<Item = u16> + '_ {
        let pointers: &[u16] = match self {
            Self::LeafIndex(p) => &p.cell_pointers,
            Self::LeafTable(p) => &p.cell_pointers,
            Self::InteriorIndex(p) => &p.cell_pointers,
            Self::InteriorTable(p) => &p.cell_pointers,
        };
        pointers.iter().copied()
    }
}
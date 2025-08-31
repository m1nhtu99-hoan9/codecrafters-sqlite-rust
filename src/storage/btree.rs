use crate::storage::page::{InteriorIndexPage, InteriorTablePage, LeafIndexPage, LeafTablePage};
use anyhow::bail;

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    Null,
    Integer { size: i64 },
    Real,
    ConstantZero,
    ConstantOne,
    Text { length: usize }, // length in bytes
    Blob { length: usize }, // length in bytes
}

impl ColumnType {
    pub fn from_serial_type(code: u64) -> anyhow::Result<Self> {
        match code {
            0 => Ok(Self::Null),
            1 => Ok(Self::Integer { size: 1 }),
            2 => Ok(Self::Integer { size: 2 }),
            3 => Ok(Self::Integer { size: 3 }),
            4 => Ok(Self::Integer { size: 4 }),
            5 => Ok(Self::Integer { size: 6 }),
            6 => Ok(Self::Integer { size: 8 }),
            7 => Ok(Self::Real),
            8 => Ok(Self::ConstantZero),
            9 => Ok(Self::ConstantOne),
            10 | 11 => bail!("Reserved serial type code: {}", code),
            n if n >= 12 && n % 2 == 0 => {
                let length = ((n - 12) / 2) as usize;
                Ok(Self::Blob { length })
            }
            n if n >= 13 && n % 2 == 1 => {
                let length = ((n - 13) / 2) as usize;
                Ok(Self::Text { length })
            }
            other => bail!("Invalid serial type code: {}", other),
        }
    }

    /// Returns the size in bytes that this column occupies in the record data
    pub fn data_size(&self) -> usize {
        match self {
            ColumnType::Null | ColumnType::ConstantZero | ColumnType::ConstantOne => 0,
            ColumnType::Integer { size } => *size as usize,
            ColumnType::Real => 8,
            ColumnType::Text { length } | ColumnType::Blob { length } => *length,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordHeader {
    pub column_types: Vec<ColumnType>,
    pub data_start_offset: usize,
}

impl RecordHeader {
    pub fn parse(buffer: &[u8], record_start: usize) -> anyhow::Result<Self> {
        let mut offset = record_start;

        // Read header size
        let (header_size, bytes_consumed) = read_varint(&buffer[offset..])?;
        offset += bytes_consumed;

        let header_end = record_start + header_size as usize;
        let mut column_types = Vec::new();

        // Read column serial types until we reach header_end
        while offset < header_end {
            let (serial_type, bytes_consumed) = read_varint(&buffer[offset..])?;
            column_types.push(ColumnType::from_serial_type(serial_type)?);
            offset += bytes_consumed;
        }

        Ok(RecordHeader {
            column_types,
            data_start_offset: header_end,
        })
    }

    pub fn column_count(&self) -> usize {
        self.column_types.len()
    }
}

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
            if offset + 1 >= buffer.len() {
                bail!("Cell pointer {} extends beyond page boundary at offset {}", i, offset);
            }
            let pointer = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
            cell_pointers.push(pointer);
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

#[derive(Debug, Clone, PartialEq)]
pub struct LeafTableCell {
    // row_id: i64, ignored for now
    pub record_header: RecordHeader,
}

impl LeafTableCell {
    pub fn parse(buffer: &[u8], cell_offset: u16) -> anyhow::Result<Self> {
        let mut offset = cell_offset as usize;

        // Parse payload size (we don't need it since overflow is out of scope)
        let (_payload_size, bytes_consumed) = read_varint(&buffer[offset..])?;
        offset += bytes_consumed;

        // Parse rowid
        let (_rowid, bytes_consumed) = read_varint(&buffer[offset..])?;
        offset += bytes_consumed;

        // Parse record header starting at current offset
        let record_header = RecordHeader::parse(buffer, offset)?;

        Ok(LeafTableCell { record_header })
    }

    /// Extract column data as raw bytes for a specific column index
    pub fn column_data<'a>(&self, buffer: &'a [u8], column_index: usize) -> anyhow::Result<&'a [u8]> {
        if column_index >= self.record_header.column_types.len() {
            bail!("Column index {} out of bounds", column_index);
        }

        let mut data_offset = self.record_header.data_start_offset;

        // Skip over previous columns to find our target column
        for i in 0..column_index {
            let column_type = &self.record_header.column_types[i];
            data_offset += column_type.data_size();
        }

        let target_type = &self.record_header.column_types[column_index];
        let data_size = target_type.data_size();

        if data_offset + data_size > buffer.len() {
            bail!("Column data extends beyond buffer");
        }

        Ok(&buffer[data_offset..data_offset + data_size])
    }

    /// Extract TEXT column as String
    pub fn text_column(&self, buffer: &[u8], column_index: usize) -> anyhow::Result<String> {
        let column_type = &self.record_header.column_types[column_index];

        match column_type {
            ColumnType::Text { length: _ } => {
                let data = self.column_data(buffer, column_index)?;
                Ok(String::from_utf8(data.to_vec())?)
            }
            _ => bail!("Column {} is not TEXT type", column_index),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaMasterRecord {
    pub type_: String,    // "table", "index", "view", etc.
    pub name: String,     // The table/index/view name
    pub tbl_name: String, // The table this belongs to
    pub rootpage: i64,    // Page number where this object's B-tree starts
    pub sql: String,      // The CREATE statement
}

impl SchemaMasterRecord {
    pub fn from_cell(buffer: &[u8], cell: &LeafTableCell) -> anyhow::Result<Self> {
        // sqlite_schema has exactly 5 columns: type, name, tbl_name, rootpage, sql
        if cell.record_header.column_count() != 5 {
            bail!(
                "Expected sqlite_schema to have 5 columns, got {}",
                cell.record_header.column_count()
            );
        }

        let type_ = cell.text_column(buffer, 0)?;
        let name = cell.text_column(buffer, 1)?;
        let tbl_name = cell.text_column(buffer, 2)?;

        // rootpage is an integer - we need to handle different integer sizes
        let rootpage = match &cell.record_header.column_types[3] {
            ColumnType::Integer { size } => {
                let data = cell.column_data(buffer, 3)?;
                match size {
                    0 => 0, // constant 0
                    1 => data[0] as i8 as i64,
                    2 => i16::from_be_bytes([data[0], data[1]]) as i64,
                    3 => {
                        // 3-byte signed integer
                        let mut bytes = [0u8; 4];
                        bytes[1..].copy_from_slice(data);
                        let val = i32::from_be_bytes(bytes);
                        // Adjust for sign extension
                        (if val & 0x800000 != 0 {
                            val | 0xFF000000u32 as i32
                        } else {
                            val
                        }) as i64
                    }
                    4 => i32::from_be_bytes([data[0], data[1], data[2], data[3]]) as i64,
                    6 => {
                        // 6-byte signed integer
                        let mut bytes = [0u8; 8];
                        bytes[2..].copy_from_slice(data);
                        let val = i64::from_be_bytes(bytes);
                        // Adjust for sign extension
                        if val & 0x800000000000 != 0 {
                            val | 0xFFFF000000000000u64 as i64
                        } else {
                            val
                        }
                    }
                    8 => i64::from_be_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]),
                    _ => bail!("Unsupported integer size: {}", size),
                }
            }
            _ => bail!("Expected rootpage to be INTEGER type"),
        };

        let sql = cell.text_column(buffer, 4)?;

        Ok(SchemaMasterRecord {
            type_,
            name,
            tbl_name,
            rootpage,
            sql,
        })
    }
}

/// Read a SQLite varint from buffer starting at given position
/// Returns (value, bytes_consumed)
fn read_varint(buffer: &[u8]) -> anyhow::Result<(u64, usize)> {
    if buffer.is_empty() {
        bail!("Cannot read varint from empty buffer");
    }

    let mut result = 0u64;
    let mut bytes_read = 0;

    for (i, &byte) in buffer.iter().enumerate() {
        if i >= 9 {
            bail!("Varint too long (max 9 bytes)");
        }

        bytes_read += 1;

        if i < 8 {
            // For bytes 0-7: use 7 bits, check continuation bit
            result = (result << 7) | ((byte & 0x7F) as u64);

            if (byte & 0x80) == 0 {
                // No continuation bit, we're done
                return Ok((result, bytes_read));
            }
        } else {
            // Byte 8: use all 8 bits (no continuation bit)
            result = (result << 8) | (byte as u64);
            return Ok((result, bytes_read));
        }
    }

    bail!("Incomplete varint in buffer");
}

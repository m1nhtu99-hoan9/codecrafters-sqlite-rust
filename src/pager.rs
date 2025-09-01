use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageNumber(u64);

impl PageNumber {
    pub fn new(value: u64) -> anyhow::Result<Self, String> {
        if value > 0 {
            Ok(PageNumber(value))
        } else {
            Err("Page number must be greater than 0".to_string())
        }
    }

    #[inline]
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(PartialEq, Debug)]
pub struct Pager<I> {
    /// [`File`] in action, [`Cursor<Vec<u8>>`] in test
    input: I,
    pub page_size: usize,
}

impl<I> Pager<I> {
    pub fn new(input: I, page_size: usize) -> Self {
        Self { input, page_size }
    }

    #[inline]
    fn guard_outbound_page(&self, page_number: PageNumber) {
        debug_assert!(
            self.page_size * (page_number.value() as usize) < (100 << 20),
            "page number {} too high for page_size {}: limit is 100 MiB",
            page_number.value(),
            self.page_size
        )
    }
}

impl<F: Seek + Read> Pager<F> {
    pub fn read(&mut self, page_number: PageNumber, buf: &mut [u8]) -> anyhow::Result<()> {
        self.guard_outbound_page(page_number);
        
        if buf.len() != self.page_size {
            anyhow::bail!("Buffer size {} does not match page size {}", buf.len(), self.page_size);
        }
        
        // Calculate file offset for the page - all pages start at page boundaries
        // Page 1: bytes 0-4095, Page 2: bytes 4096-8191, etc.
        let file_offset = (page_number.value() - 1) * (self.page_size as u64);
        
        // Seek to the page location and read
        self.input.seek(SeekFrom::Start(file_offset))?;
        self.input.read_exact(buf)?;
        
        Ok(())
    }
}

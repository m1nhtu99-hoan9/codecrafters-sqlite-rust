use std::io::{Read, Seek};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageNumber(u64);

impl PageNumber {
    pub fn new(value: u64) -> Result<Self, String> {
        if value > 0 {
            Ok(PageNumber(value))
        } else {
            Err("Page number must be greater than 0".to_string())
        }
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(PartialEq, Debug)]
pub struct Pager<I> {
    /// `File` in action, `Cursor<Vec<u8>>` in test
    pub input: I,
    pub page_size: usize,
}

impl<I> Pager<I> {
    pub fn new(file: I, page_size: usize) -> Self {
        Self { input: file, page_size }
    }

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
    pub fn read(&mut self, page_number: PageNumber, _buf: &mut [u8])  {
        self.guard_outbound_page(page_number);
        todo!()
    }
}

use super::file::{FdFlags, File, FileType, ReadWrite};
use anyhow::Result;
use std::io::Cursor;

pub struct VirtualFile(Box<dyn ReadWrite>);

impl File for VirtualFile {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let written = self.0.write(data)?;
        Ok(written)
    }

    fn read(&mut self, data: &mut [u8]) -> Result<usize> {
        Ok(self.0.read(data)?)
    }

    fn seek(&mut self, pos: u64) -> Result<u64> {
        Ok(self.0.seek(std::io::SeekFrom::Start(pos))?)
    }

    fn read_string(&mut self) -> Result<String> {
        let mut buf = String::new();
        self.0.read_to_string(&mut buf)?;
        Ok(buf)
    }

    fn filetype(&self) -> Result<super::file::FileType> {
        Ok(FileType::RegularFile)
    }

    fn fdflags(&self) -> Result<super::file::FdFlags> {
        Ok(FdFlags::Append)
    }
}

impl Default for VirtualFile {
    fn default() -> Self {
        Self(Box::new(Cursor::new(vec![])))
    }
}

impl VirtualFile {
    pub fn new(data: &[u8]) -> Self {
        Self(Box::new(Cursor::new(data.to_vec())))
    }
}

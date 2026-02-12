use super::file::{FdFlags, File, FileType};
use anyhow::Result;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub struct VirtualFile(Cursor<Vec<u8>>);

impl File for VirtualFile {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let written = self.0.write(data)?;
        Ok(written)
    }

    fn read(&mut self, data: &mut [u8]) -> Result<usize> {
        Ok(self.0.read(data)?)
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        Ok(self.0.seek(pos)?)
    }

    fn set_len(&mut self, len: u64) -> Result<()> {
        let len: usize = len.try_into()?;
        let data = self.0.get_mut();
        data.resize(len, 0);
        Ok(())
    }

    fn size(&self) -> Result<u64> {
        Ok(self.0.get_ref().len() as u64)
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
        Self(Cursor::new(vec![]))
    }
}

impl VirtualFile {
    pub fn new(data: &[u8]) -> Self {
        Self(Cursor::new(data.to_vec()))
    }
}

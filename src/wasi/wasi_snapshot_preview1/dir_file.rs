use super::file::{FdFlags, File, FileType};
use anyhow::Result;
use std::io::SeekFrom;

pub struct DirFile;

impl File for DirFile {
    fn write(&mut self, _data: &[u8]) -> Result<usize> {
        Ok(0)
    }

    fn read(&mut self, _data: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        Ok(0)
    }

    fn filetype(&self) -> Result<FileType> {
        Ok(FileType::Directory)
    }

    fn fdflags(&self) -> Result<FdFlags> {
        Ok(FdFlags::Append)
    }

    fn read_string(&mut self) -> Result<String> {
        Ok(String::new())
    }
}

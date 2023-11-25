use super::file::{FdFlags, File, FileType};
use anyhow::Result;
use std::{io::prelude::*, os::fd::FromRawFd};

pub struct WasiFile(std::fs::File);

impl File for WasiFile {
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

    fn filetype(&self) -> Result<FileType> {
        // FIXME: this is not correct
        let m = self.0.metadata()?;
        let filetype = if m.is_file() {
            FileType::RegularFile
        } else if m.is_dir() {
            FileType::Directory
        } else if m.is_symlink() {
            FileType::SymbolicLink
        } else {
            FileType::Unknown
        };
        Ok(filetype)
    }

    fn fdflags(&self) -> Result<FdFlags> {
        // TODO: implement fdflags
        Ok(FdFlags::Append)
    }
}

impl WasiFile {
    pub fn from_raw_fd(fd: u32) -> Self {
        let file = unsafe { std::fs::File::from_raw_fd(fd as i32) };
        Self(file)
    }
}

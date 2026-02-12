use anyhow::Result;
use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

pub trait ReadWrite: Read + Write + Seek + Send + Sync + 'static {}

impl<IO: Read + Write + Seek + Send + Sync + 'static> ReadWrite for IO {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FdFlags {
    Append = 0b1,
    Dsync = 0b10,
    Nonblock = 0b1000,
    Rsync = 0b10000,
    Sync = 0b100000,
}

#[derive(Debug, Clone)]
pub enum FileCaps {
    DataSync = 0b1,
    Read = 0b10,
    Seek = 0b100,
    FdstatSetFlags = 0b1000,
    Sync = 0b10000,
    Tell = 0b100000,
    Write = 0b1000000,
    Advise = 0b10000000,
    Allocate = 0b100000000,
    FilestatGet = 0b1000000000,
    FilestatSetSize = 0b10000000000,
    FilestatSetTimes = 0b100000000000,
    PollReadwrite = 0b1000000000000,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FileType {
    Unknown = 0,
    BlockDevice = 1,
    CharacterDevice = 2,
    Directory = 3,
    RegularFile = 4,
    SocketDgram = 5,
    SocketStream = 6,
    SymbolicLink = 7,
    Pipe = 8,
}

pub trait File: Send + Sync {
    fn write(&mut self, data: &[u8]) -> Result<usize>;
    fn read(&mut self, data: &mut [u8]) -> Result<usize>;
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;
    fn tell(&mut self) -> Result<u64> {
        self.seek(SeekFrom::Current(0))
    }
    fn set_len(&mut self, _len: u64) -> Result<()> {
        Ok(())
    }
    fn size(&self) -> Result<u64> {
        Ok(0)
    }
    fn filetype(&self) -> Result<FileType>;
    fn fdflags(&self) -> Result<FdFlags>;
    fn read_string(&mut self) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct FdStat {
    pub filetype: FileType,
    pub caps: FileCaps,
    pub flags: FdFlags,
}

pub struct FileEntry {
    caps: FileCaps,
    file: Box<dyn File>,
    fdflags: FdFlags,
    preopen_path: Option<PathBuf>,
}

impl FileEntry {
    pub fn new(file: Box<dyn File>, caps: FileCaps) -> Self {
        Self {
            caps,
            file,
            fdflags: FdFlags::Append,
            preopen_path: None,
        }
    }

    pub fn with_preopen_path(mut self, path: PathBuf) -> Self {
        self.preopen_path = Some(path);
        self
    }

    pub fn get_fdstat(&self) -> Result<FdStat> {
        Ok(FdStat {
            filetype: self.file.filetype()?,
            caps: self.caps.clone(),
            flags: self.fdflags,
        })
    }

    pub fn capbable(&mut self, _cap: FileCaps) -> Result<&mut Box<dyn File>> {
        // TODO: check capabilites
        let file = &mut self.file;
        Ok(file)
    }

    pub fn preopen_path(&self) -> Option<&PathBuf> {
        self.preopen_path.as_ref()
    }

    pub fn set_fdflags(&mut self, flags: FdFlags) {
        self.fdflags = flags;
    }
}

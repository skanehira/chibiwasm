use anyhow::Result;
use std::fs;
use std::io::{Cursor, Read, Seek, Write};
use std::os::unix::io::FromRawFd;
use std::sync::{Arc, Mutex};

pub trait ReadWrite: Read + Write + Seek {}

impl<IO: Read + Write + Send + Seek> ReadWrite for IO {}

// ref: https://github.com/bytecodealliance/wasi/blob/9ec04a7d8ebb1bbb9e3291503425cee1ec38a560/src/lib_generated.rs#L554-L570
pub struct Filetype(u8);

pub const FILETYPE_CHARACTER_DEVICE: Filetype = Filetype(2);
// TODO: impl for other file types

// ref: https://github.com/bytecodealliance/wasi/blob/9ec04a7d8ebb1bbb9e3291503425cee1ec38a560/src/lib_generated.rs#L660-L672
pub type Fdflags = u16;

pub const FDFLAGS_APPEND: Fdflags = 1 << 0;
pub const FDFLAGS_DSYNC: Fdflags = 1 << 1;
pub const FDFLAGS_NONBLOCK: Fdflags = 1 << 2;
pub const FDFLAGS_RSYNC: Fdflags = 1 << 3;
pub const FDFLAGS_SYNC: Fdflags = 1 << 4;

// ref: https://github.com/bytecodealliance/wasi/blob/9ec04a7d8ebb1bbb9e3291503425cee1ec38a560/src/lib_generated.rs#L414-L486
pub type Rights = u64;

// TODO: impl for rights

pub struct FileDescriptor {
    file_type: Filetype,
    flags: u32,
    rights_base: Rights,
    rights_inheriting: Rights,
    inner: Box<dyn ReadWrite>,
}

impl FileDescriptor {
    pub fn from_raw_fd(fd: u32) -> Result<Self> {
        let file = unsafe { fs::File::from_raw_fd(fd as i32) };
        Ok(Self {
            file_type: todo!(),
            flags: todo!(),
            rights_base: 0,       // TODO
            rights_inheriting: 0, // TODO
            inner: Box::new(file),
        })
    }
}

impl Write for FileDescriptor {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(data)?;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl Read for FileDescriptor {
    fn read(&mut self, data: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(data)
    }
}

impl Seek for FileDescriptor {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.inner.rewind()
    }
}

pub struct File(Box<dyn ReadWrite>);

impl File {
    pub fn from_buffer(buffer: Vec<u8>) -> Self {
        File(Box::new(Cursor::new(buffer)))
    }

    pub fn from_raw_fd(fd: u32) -> Self {
        let file = unsafe { fs::File::from_raw_fd(fd as i32) };
        File(Box::new(file))
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        let written = self.0.write(data)?;
        Ok(written)
    }

    pub fn read(&mut self, data: &mut [u8]) -> Result<usize> {
        Ok(self.0.read(data)?)
    }

    pub fn seek(&mut self, pos: u64) -> Result<u64> {
        Ok(self.0.seek(std::io::SeekFrom::Start(pos))?)
    }

    pub fn read_string(&mut self) -> Result<String> {
        let mut buf = String::new();
        self.0.read_to_string(&mut buf)?;
        Ok(buf)
    }
}

pub struct FileTable(Vec<Arc<Mutex<File>>>);

impl Default for FileTable {
    fn default() -> Self {
        Self(vec![
            Arc::new(Mutex::new(File::from_raw_fd(0))), // stdin
            Arc::new(Mutex::new(File::from_raw_fd(1))), // stdout
            Arc::new(Mutex::new(File::from_raw_fd(2))), // stderr
        ])
    }
}

impl FileTable {
    pub fn with_io(files: Vec<Arc<Mutex<File>>>) -> Self {
        let mut file_table = FileTable(vec![]);
        for file in files {
            file_table.add(file);
        }
        file_table
    }
    pub fn get(&self, idx: usize) -> Option<&Arc<Mutex<File>>> {
        self.0.get(idx)
    }
    pub fn add(&mut self, file: Arc<Mutex<File>>) {
        self.0.push(file);
    }
}

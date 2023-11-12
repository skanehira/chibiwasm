use anyhow::Result;
use std::fs;
use std::io::{Cursor, Read, Seek, Write};
use std::os::unix::io::FromRawFd;
use std::sync::{Arc, Mutex};

pub trait ReadWrite: Read + Write + Seek {}

impl<IO: Read + Write + Send + Seek> ReadWrite for IO {}

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

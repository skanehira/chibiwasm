use super::{
    file::{FileCaps, FileEntry},
    wasi_file::WasiFile,
};
use std::sync::{Arc, Mutex};

pub struct FileTable(Vec<Arc<Mutex<FileEntry>>>);

impl Default for FileTable {
    fn default() -> Self {
        Self(vec![
            // stdin
            Arc::new(Mutex::new(FileEntry::new(
                Box::new(WasiFile::from_raw_fd(0)),
                FileCaps::Sync,
            ))),
            // stdout
            Arc::new(Mutex::new(FileEntry::new(
                Box::new(WasiFile::from_raw_fd(1)),
                FileCaps::Sync,
            ))),
            // stderr
            Arc::new(Mutex::new(FileEntry::new(
                Box::new(WasiFile::from_raw_fd(2)),
                FileCaps::Sync,
            ))),
        ])
    }
}

impl FileTable {
    pub fn with_io(files: Vec<Arc<Mutex<FileEntry>>>) -> Self {
        FileTable(files)
    }

    pub fn get(&self, idx: usize) -> Option<&Arc<Mutex<FileEntry>>> {
        self.0.get(idx)
    }

    pub fn add(&mut self, file: Arc<Mutex<FileEntry>>) {
        self.0.push(file);
    }
}

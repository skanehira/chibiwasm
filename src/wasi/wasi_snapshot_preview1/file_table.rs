use super::{
    file::{FileCaps, FileEntry},
    wasi_file::WasiFile,
};
use std::sync::{Arc, Mutex};

pub struct FileTable(Vec<Option<Arc<Mutex<FileEntry>>>>);

impl Default for FileTable {
    fn default() -> Self {
        Self(vec![
            // stdin
            Some(Arc::new(Mutex::new(FileEntry::new(
                Box::new(WasiFile::from_raw_fd(0)),
                FileCaps::Sync,
            )))),
            // stdout
            Some(Arc::new(Mutex::new(FileEntry::new(
                Box::new(WasiFile::from_raw_fd(1)),
                FileCaps::Sync,
            )))),
            // stderr
            Some(Arc::new(Mutex::new(FileEntry::new(
                Box::new(WasiFile::from_raw_fd(2)),
                FileCaps::Sync,
            )))),
        ])
    }
}

impl FileTable {
    pub fn with_io(files: Vec<Arc<Mutex<FileEntry>>>) -> Self {
        FileTable(files.into_iter().map(Some).collect())
    }

    pub fn get(&self, idx: usize) -> Option<&Arc<Mutex<FileEntry>>> {
        self.0.get(idx).and_then(|v| v.as_ref())
    }

    pub fn add(&mut self, file: Arc<Mutex<FileEntry>>) -> usize {
        if let Some((idx, slot)) = self.0.iter_mut().enumerate().find(|(_, s)| s.is_none()) {
            *slot = Some(file);
            return idx;
        }
        self.0.push(Some(file));
        self.0.len() - 1
    }

    pub fn take(&mut self, idx: usize) -> Option<Arc<Mutex<FileEntry>>> {
        if let Some(slot) = self.0.get_mut(idx) {
            return slot.take();
        }
        None
    }

    pub fn set(&mut self, idx: usize, file: Option<Arc<Mutex<FileEntry>>>) {
        if idx >= self.0.len() {
            self.0.resize_with(idx + 1, || None);
        }
        self.0[idx] = file;
    }

    pub fn close(&mut self, idx: usize) -> bool {
        if let Some(slot) = self.0.get_mut(idx) {
            *slot = None;
            return true;
        }
        false
    }
}

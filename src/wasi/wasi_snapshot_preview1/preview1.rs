use super::{
    dir_file::DirFile,
    file::FileEntry,
    file_table::FileTable,
    types::*,
    wasi_file::WasiFile,
};
use crate::{
    binary::instruction::MemoryArg, memory_load, memory_write, module::ExternalFuncInst,
    wasi::file::FileCaps, Importer, Store, Value,
};
use anyhow::{bail, Context as _, Result};
use log::debug;
use rand::prelude::*;
use std::{
    cell::RefCell,
    fs,
    io::SeekFrom,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct WasiSnapshotPreview1 {
    file_table: RefCell<FileTable>,
    preopen_root: PathBuf,
    preopen_root_canon: PathBuf,
    preopen_name: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl Default for WasiSnapshotPreview1 {
    fn default() -> Self {
        let file_table = FileTable::default();
        let preopen_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let preopen_root_canon = preopen_root
            .canonicalize()
            .unwrap_or_else(|_| preopen_root.clone());
        let preopen_name = "/".to_string();
        let mut wasi = Self {
            file_table: RefCell::new(file_table),
            preopen_root,
            preopen_root_canon,
            preopen_name,
            args: std::env::args().collect(),
            env: std::env::vars().collect(),
        };
        wasi.add_preopen_dir();
        wasi
    }
}

impl Importer for WasiSnapshotPreview1 {
    fn name(&self) -> &str {
        "wasi_snapshot_preview1"
    }

    fn invoke(
        &self,
        store: Rc<RefCell<Store>>,
        func: ExternalFuncInst,
        args: Vec<Value>,
    ) -> Result<Option<Value>> {
        debug!("wasi call: {}", func.field);
        let value = match func.field.as_str() {
            "fd_read" => self.fd_read(store, args),
            "fd_write" => self.fd_write(store, args),
            "proc_exit" => {
                self.proc_exit(args);
            }
            "environ_get" => self.environ_get(store, args),
            "environ_sizes_get" => self.environ_sizes_get(store, args),
            "args_get" => self.args_get(store, args),
            "args_sizes_get" => self.args_sizes_get(store, args),
            "random_get" => self.random_get(store, args),
            "fd_fdstat_get" => self.fd_fdstat_get(store, args),
            "fd_fdstat_set_flags" => self.fd_fdstat_set_flags(store, args),
            "fd_filestat_get" => self.fd_filestat_get(store, args),
            "fd_filestat_set_size" => self.fd_filestat_set_size(store, args),
            "fd_close" => self.fd_close(store, args),
            "fd_seek" => self.fd_seek(store, args),
            "fd_tell" => self.fd_tell(store, args),
            "fd_pread" => self.fd_pread(store, args),
            "fd_pwrite" => self.fd_pwrite(store, args),
            "fd_datasync" => self.fd_datasync(store, args),
            "fd_sync" => self.fd_sync(store, args),
            "fd_advise" => self.fd_advise(store, args),
            "fd_renumber" => self.fd_renumber(store, args),
            "fd_readdir" => self.fd_readdir(store, args),
            "fd_prestat_get" => self.fd_prestat_get(store, args),
            "fd_prestat_dir_name" => self.fd_prestat_dir_name(store, args),
            "path_open" => self.path_open(store, args),
            "path_filestat_get" => self.path_filestat_get(store, args),
            "path_filestat_set_times" => self.path_filestat_set_times(store, args),
            "path_create_directory" => self.path_create_directory(store, args),
            "path_remove_directory" => self.path_remove_directory(store, args),
            "path_unlink_file" => self.path_unlink_file(store, args),
            "path_rename" => self.path_rename(store, args),
            "path_link" => self.path_link(store, args),
            "path_symlink" => self.path_symlink(store, args),
            "path_readlink" => self.path_readlink(store, args),
            "clock_res_get" => self.clock_res_get(store, args),
            "clock_time_get" => self.clock_time_get(store, args),
            "poll_oneoff" => self.poll_oneoff(store, args),
            _ => bail!("unimplemented wasi function: {}", func.field),
        }?;
        if let Value::I32(code) = value {
            debug!("wasi ret: {} -> {}", func.field, code);
        }
        Ok(Some(value))
    }
}

impl WasiSnapshotPreview1 {
    pub fn with_io(files: Vec<Arc<Mutex<FileEntry>>>) -> Self {
        let file_table = FileTable::with_io(files);
        let preopen_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let preopen_root_canon = preopen_root
            .canonicalize()
            .unwrap_or_else(|_| preopen_root.clone());
        let preopen_name = "/".to_string();
        let mut wasi = Self {
            file_table: RefCell::new(file_table),
            preopen_root,
            preopen_root_canon,
            preopen_name,
            args: std::env::args().collect(),
            env: std::env::vars().collect(),
        };
        wasi.add_preopen_dir();
        wasi
    }

    pub fn set_args_env(&mut self, args: Vec<String>, env: Vec<(String, String)>) {
        self.args = args;
        self.env = env;
    }

    fn add_preopen_dir(&mut self) {
        let dir = FileEntry::new(Box::new(DirFile), FileCaps::Sync)
            .with_preopen_path(self.preopen_root.clone());
        let dir = Arc::new(Mutex::new(dir));
        self.file_table.borrow_mut().add(dir);
    }

    fn file_arc(&self, fd: usize) -> Option<Arc<Mutex<FileEntry>>> {
        let table = self.file_table.borrow();
        let file = table.get(fd)?;
        Some(Arc::clone(file))
    }

    fn proc_exit(&self, args: Vec<Value>) -> ! {
        let exit_code: i32 = args
            .first()
            .expect("no any argument in proc_exit")
            .clone()
            .into();
        std::process::exit(exit_code);
    }

    fn environ_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (mut offset, mut buf_offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let env = self.env.clone();
        if env.is_empty() {
            // Some runtimes assume environ has at least one entry.
            if offset + 4 > memory.data.len() {
                return Ok((ERRNO_INVAL as i32).into());
            }
            if buf_offset + 1 > memory.data.len() {
                return Ok((ERRNO_INVAL as i32).into());
            }
            memory_write!(memory, 0, 4, offset, buf_offset);
            memory.write_bytes(buf_offset, b"\0")?;
            return Ok(0.into());
        }
        for (key, val) in env {
            if offset + 4 > memory.data.len() {
                return Ok((ERRNO_INVAL as i32).into());
            }
            memory_write!(memory, 0, 4, offset, buf_offset);
            offset += 4;

            let data = format!("{}={}\0", key, val);
            let data = data.as_bytes();

            if buf_offset + data.len() > memory.data.len() {
                return Ok((ERRNO_INVAL as i32).into());
            }
            memory.write_bytes(buf_offset, data)?;
            buf_offset += data.len();
        }
        Ok(0.into())
    }

    fn environ_sizes_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (offset, buf_offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let env = self.env.clone();
        if env.is_empty() {
            memory_write!(memory, 0, 4, offset, 1);
            memory_write!(memory, 0, 4, buf_offset, 1);
            return Ok(0.into());
        }
        memory_write!(memory, 0, 4, offset, env.len());

        let size = env.iter().fold(0, |acc, (key, val)| {
            let data = format!("{}={}\0", key, val);
            acc + data.as_bytes().len()
        });

        memory_write!(memory, 0, 4, buf_offset, size);

        Ok(0.into())
    }

    fn fd_read(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (fd, mut iovs, iovs_len, nread_offset) = (
            args[0] as usize,
            args[1] as usize,
            args[2] as usize,
            args[3] as usize,
        );

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let mut file = file.lock().expect("cannot lock file");
        let file = file.capbable(FileCaps::Read)?;

        let mut nread = 0;
        for _ in 0..iovs_len {
            let offset: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let len: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let offset = offset as usize;
            let end = offset + len as usize;

            nread += file.read(&mut memory.data[offset..end])?;
        }

        memory_write!(memory, 0, 4, nread_offset, nread);

        Ok(0.into())
    }

    fn fd_write(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (fd, mut iovs, iovs_len, rp) = (
            args[0] as usize,
            args[1] as usize,
            args[2] as usize,
            args[3] as usize,
        );

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };

        let mut file = file.lock().expect("cannot lock file");
        let file = file.capbable(FileCaps::Write)?;

        let mut written = 0;

        for _ in 0..iovs_len {
            let offset: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let len: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let offset = offset as usize;
            let end = offset + len as usize;
            let buf = &memory.data[offset..end];

            written += file.write(buf)?;
        }

        memory_write!(memory, 0, 4, rp, written);

        Ok(0.into())
    }

    fn args_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (mut offset, mut buf_offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let args = self.args.clone();
        debug!("args_get: {:?}", args);
        for arg in args {
            if offset + 4 > memory.data.len() {
                return Ok((ERRNO_INVAL as i32).into());
            }
            memory_write!(memory, 0, 4, offset, buf_offset);
            offset += 4;

            let data = format!("{}\0", arg);
            let data = data.as_bytes();

            if buf_offset + data.len() > memory.data.len() {
                return Ok((ERRNO_INVAL as i32).into());
            }
            memory.write_bytes(buf_offset, data)?;
            buf_offset += data.len();
        }

        Ok(0.into())
    }

    fn args_sizes_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (offset, buf_offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let args = self.args.clone();
        debug!("args_sizes_get: {:?}", args);
        memory_write!(memory, 0, 4, offset, args.len());

        let size = args.iter().fold(0, |acc, arg| {
            let data = format!("{}\0", arg);
            acc + data.as_bytes().len()
        });

        memory_write!(memory, 0, 4, buf_offset, size);

        Ok(0.into())
    }

    fn random_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (offset, buf_len) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let mut rng = thread_rng();
        let mut buf = vec![0u8; buf_len];
        rng.fill_bytes(&mut buf);
        memory.write_bytes(offset, &buf)?;

        Ok(0.into())
    }

    fn fd_fdstat_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (fd, offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let file = file.lock().expect("cannot lock file");
        let stat = file.get_fdstat()?;

        write_u8(&mut memory, offset, stat.filetype as u8)?;
        write_u16(&mut memory, offset + 2, stat.flags as u16)?;
        write_u64(&mut memory, offset + 8, u64::MAX)?;
        write_u64(&mut memory, offset + 16, u64::MAX)?;

        Ok(0.into())
    }

    fn fd_fdstat_set_flags(&self, _store: Rc<RefCell<Store>>, _args: Vec<Value>) -> Result<Value> {
        Ok(0.into())
    }

    fn fd_filestat_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let offset = arg_i32(&args, 1) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let mut file = file.lock().expect("cannot lock file");

        let stat = file.get_fdstat()?;
        let (size, filetype, atim, mtim, ctim) = if let Some(path) = file.preopen_path() {
            let meta = fs::symlink_metadata(path)?;
            (
                meta.len(),
                stat.filetype,
                time_to_nanos(meta.accessed().ok()),
                time_to_nanos(meta.modified().ok()),
                time_to_nanos(meta.created().ok()),
            )
        } else {
            (
                file.capbable(FileCaps::Read)?.size()?,
                stat.filetype,
                0,
                0,
                0,
            )
        };

        write_u64(&mut memory, offset + 0, 0)?;
        write_u64(&mut memory, offset + 8, 0)?;
        write_u8(&mut memory, offset + 16, filetype as u8)?;
        write_u64(&mut memory, offset + 24, 0)?;
        write_u64(&mut memory, offset + 32, size)?;
        write_u64(&mut memory, offset + 40, atim)?;
        write_u64(&mut memory, offset + 48, mtim)?;
        write_u64(&mut memory, offset + 56, ctim)?;

        Ok(0.into())
    }

    fn fd_filestat_set_size(&self, _store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let size = arg_i64(&args, 1) as u64;

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let mut file = file.lock().expect("cannot lock file");
        let file = file.capbable(FileCaps::Write)?;
        file.set_len(size)?;

        Ok(0.into())
    }

    fn fd_close(&self, _store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let ok = self.file_table.borrow_mut().close(fd);
        if ok {
            Ok(0.into())
        } else {
            Ok((ERRNO_BADF as i32).into())
        }
    }

    fn fd_seek(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let offset = arg_i64(&args, 1);
        let whence = arg_i32(&args, 2);
        let newoffset = arg_i32(&args, 3) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let mut file = file.lock().expect("cannot lock file");
        let file = file.capbable(FileCaps::Seek)?;

        let pos = match whence {
            0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::Current(offset),
            2 => SeekFrom::End(offset),
            _ => SeekFrom::Start(0),
        };
        let new_pos = file.seek(pos)?;
        write_u64(&mut memory, newoffset, new_pos)?;

        Ok(0.into())
    }

    fn fd_tell(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let offset = arg_i32(&args, 1) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let mut file = file.lock().expect("cannot lock file");
        let file = file.capbable(FileCaps::Tell)?;
        let pos = file.tell()?;
        write_u64(&mut memory, offset, pos)?;

        Ok(0.into())
    }

    fn fd_pread(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let mut iovs = arg_i32(&args, 1) as usize;
        let iovs_len = arg_i32(&args, 2) as usize;
        let offset = arg_i64(&args, 3) as u64;
        let nread_offset = arg_i32(&args, 4) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let mut file = file.lock().expect("cannot lock file");
        let file = file.capbable(FileCaps::Read)?;

        let current = file.tell()?;
        file.seek(SeekFrom::Start(offset))?;

        let mut nread = 0;
        for _ in 0..iovs_len {
            let buf_offset: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;
            let len: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let buf_offset = buf_offset as usize;
            let end = buf_offset + len as usize;
            nread += file.read(&mut memory.data[buf_offset..end])?;
        }

        file.seek(SeekFrom::Start(current))?;
        memory_write!(memory, 0, 4, nread_offset, nread);

        Ok(0.into())
    }

    fn fd_pwrite(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let mut iovs = arg_i32(&args, 1) as usize;
        let iovs_len = arg_i32(&args, 2) as usize;
        let offset = arg_i64(&args, 3) as u64;
        let nwritten_offset = arg_i32(&args, 4) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let mut file = file.lock().expect("cannot lock file");
        let file = file.capbable(FileCaps::Write)?;

        let current = file.tell()?;
        file.seek(SeekFrom::Start(offset))?;

        let mut written = 0;
        for _ in 0..iovs_len {
            let buf_offset: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;
            let len: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let buf_offset = buf_offset as usize;
            let end = buf_offset + len as usize;
            let buf = &memory.data[buf_offset..end];
            written += file.write(buf)?;
        }

        file.seek(SeekFrom::Start(current))?;
        memory_write!(memory, 0, 4, nwritten_offset, written);

        Ok(0.into())
    }

    fn fd_datasync(&self, _store: Rc<RefCell<Store>>, _args: Vec<Value>) -> Result<Value> {
        Ok(0.into())
    }

    fn fd_sync(&self, _store: Rc<RefCell<Store>>, _args: Vec<Value>) -> Result<Value> {
        Ok(0.into())
    }

    fn fd_advise(&self, _store: Rc<RefCell<Store>>, _args: Vec<Value>) -> Result<Value> {
        Ok(0.into())
    }

    fn fd_renumber(&self, _store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let from = arg_i32(&args, 0) as usize;
        let to = arg_i32(&args, 1) as usize;

        let mut table = self.file_table.borrow_mut();
        let from_entry = match table.take(from) {
            Some(entry) => entry,
            None => return Ok((ERRNO_BADF as i32).into()),
        };

        table.set(to, Some(from_entry));

        Ok(0.into())
    }

    fn fd_readdir(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let buf = arg_i32(&args, 1) as usize;
        let buf_len = arg_i32(&args, 2) as usize;
        let cookie = arg_i64(&args, 3) as u64;
        let bufused = arg_i32(&args, 4) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let file = file.lock().expect("cannot lock file");
        let Some(dir_path) = file.preopen_path() else {
            return Ok((ERRNO_NOTDIR as i32).into());
        };

        let mut used = 0usize;
        let mut index = 0u64;

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            if index < cookie {
                index += 1;
                continue;
            }

            let name = entry.file_name();
            let name = name.to_string_lossy();
            let name_bytes = name.as_bytes();
            let entry_size = 24 + name_bytes.len();
            if used + entry_size > buf_len {
                break;
            }

            let offset = buf + used;
            write_u64(&mut memory, offset + 0, index + 1)?;
            write_u64(&mut memory, offset + 8, 0)?;
            write_u32(&mut memory, offset + 16, name_bytes.len() as u32)?;
            write_u8(&mut memory, offset + 20, filetype_to_u8(&entry.file_type()?))?;
            memory.write_bytes(offset + 24, name_bytes)?;

            used += entry_size;
            index += 1;
        }

        write_u32(&mut memory, bufused, used as u32)?;

        Ok(0.into())
    }

    fn fd_prestat_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let offset = arg_i32(&args, 1) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            debug!("fd_prestat_get: fd={} not found", fd);
            return Ok((ERRNO_BADF as i32).into());
        };
        let file = file.lock().expect("cannot lock file");

        if file.preopen_path().is_none() {
            debug!("fd_prestat_get: fd={} not preopen", fd);
            return Ok((ERRNO_NOTCAPABLE as i32).into());
        }

        write_u8(&mut memory, offset, 0)?;
        write_u32(&mut memory, offset + 4, self.preopen_name.len() as u32)?;

        Ok(0.into())
    }

    fn fd_prestat_dir_name(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let fd = arg_i32(&args, 0) as usize;
        let path_ptr = arg_i32(&args, 1) as usize;
        let path_len = arg_i32(&args, 2) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let Some(file) = self.file_arc(fd) else {
            return Ok((ERRNO_BADF as i32).into());
        };
        let file = file.lock().expect("cannot lock file");

        if file.preopen_path().is_none() {
            return Ok((ERRNO_NOTCAPABLE as i32).into());
        }

        let name = self.preopen_name.as_bytes();
        if path_len < name.len() {
            return Ok((ERRNO_INVAL as i32).into());
        }
        memory.write_bytes(path_ptr, name)?;

        Ok(0.into())
    }

    fn path_open(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let dirfd = arg_i32(&args, 0) as usize;
        let _dirflags = arg_i32(&args, 1);
        let path_ptr = arg_i32(&args, 2) as usize;
        let path_len = arg_i32(&args, 3) as usize;
        let oflags = arg_i32(&args, 4) as u16;
        let _rights_base = arg_i64(&args, 5) as u64;
        let _rights_inherit = arg_i64(&args, 6) as u64;
        let _fdflags = arg_i32(&args, 7) as u16;
        let result_fd = arg_i32(&args, 8) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let path = read_string(&memory, path_ptr, path_len)?;
        debug!("path_open: path={} oflags={}", path, oflags);
        let full_path = match self.resolve_path_from(dirfd, &path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };

        let file_entry = if oflags & OFLAGS_DIRECTORY != 0 {
            FileEntry::new(Box::new(DirFile), FileCaps::Sync).with_preopen_path(full_path)
        } else {
            let mut opts = fs::OpenOptions::new();
            opts.read(true).write(true);
            if oflags & OFLAGS_CREAT != 0 {
                opts.create(true);
            }
            if oflags & OFLAGS_TRUNC != 0 {
                opts.truncate(true);
            }
            if oflags & OFLAGS_EXCL != 0 {
                opts.create_new(true);
            }
            let file = match opts.open(&full_path) {
                Ok(file) => file,
                Err(_) => return Ok((ERRNO_NOENT as i32).into()),
            };
            FileEntry::new(Box::new(WasiFile::from_file(file)), FileCaps::Sync)
        };

        let fd = self
            .file_table
            .borrow_mut()
            .add(Arc::new(Mutex::new(file_entry)));

        write_u32(&mut memory, result_fd, fd as u32)?;

        Ok(0.into())
    }

    fn path_filestat_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let dirfd = arg_i32(&args, 0) as usize;
        let _flags = arg_i32(&args, 1) as u32;
        let path_ptr = arg_i32(&args, 2) as usize;
        let path_len = arg_i32(&args, 3) as usize;
        let buf = arg_i32(&args, 4) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let path = read_string(&memory, path_ptr, path_len)?;
        debug!("path_filestat_get: path={}", path);
        let full_path = match self.resolve_path_from(dirfd, &path) {
            Ok(path) => path,
            Err(err) => {
                debug!("path_filestat_get: resolve error: {}", err);
                return Ok((ERRNO_NOENT as i32).into());
            }
        };
        let meta = match fs::symlink_metadata(&full_path) {
            Ok(meta) => meta,
            Err(err) => {
                debug!("path_filestat_get: meta error: {}", err);
                return Ok((ERRNO_NOENT as i32).into());
            }
        };

        write_u64(&mut memory, buf + 0, 0)?;
        write_u64(&mut memory, buf + 8, 0)?;
        write_u8(&mut memory, buf + 16, filetype_to_u8(&meta.file_type()))?;
        write_u64(&mut memory, buf + 24, 0)?;
        write_u64(&mut memory, buf + 32, meta.len())?;
        write_u64(&mut memory, buf + 40, time_to_nanos(meta.accessed().ok()))?;
        write_u64(&mut memory, buf + 48, time_to_nanos(meta.modified().ok()))?;
        write_u64(&mut memory, buf + 56, time_to_nanos(meta.created().ok()))?;

        Ok(0.into())
    }

    fn path_filestat_set_times(
        &self,
        _store: Rc<RefCell<Store>>,
        _args: Vec<Value>,
    ) -> Result<Value> {
        Ok(0.into())
    }

    fn path_create_directory(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let dirfd = arg_i32(&args, 0) as usize;
        let path_ptr = arg_i32(&args, 1) as usize;
        let path_len = arg_i32(&args, 2) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let memory = memory.borrow();

        let path = read_string(&memory, path_ptr, path_len)?;
        let full_path = match self.resolve_path_from(dirfd, &path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        if fs::create_dir(&full_path).is_err() {
            return Ok((ERRNO_NOENT as i32).into());
        }

        Ok(0.into())
    }

    fn path_remove_directory(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let dirfd = arg_i32(&args, 0) as usize;
        let path_ptr = arg_i32(&args, 1) as usize;
        let path_len = arg_i32(&args, 2) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let memory = memory.borrow();

        let path = read_string(&memory, path_ptr, path_len)?;
        let full_path = match self.resolve_path_from(dirfd, &path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        if fs::remove_dir(&full_path).is_err() {
            return Ok((ERRNO_NOENT as i32).into());
        }

        Ok(0.into())
    }

    fn path_unlink_file(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let dirfd = arg_i32(&args, 0) as usize;
        let path_ptr = arg_i32(&args, 1) as usize;
        let path_len = arg_i32(&args, 2) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let memory = memory.borrow();

        let path = read_string(&memory, path_ptr, path_len)?;
        let full_path = match self.resolve_path_from(dirfd, &path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        if fs::remove_file(&full_path).is_err() {
            return Ok((ERRNO_NOENT as i32).into());
        }

        Ok(0.into())
    }

    fn path_rename(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let _old_dirfd = arg_i32(&args, 0) as usize;
        let old_path_ptr = arg_i32(&args, 1) as usize;
        let old_path_len = arg_i32(&args, 2) as usize;
        let _new_dirfd = arg_i32(&args, 3) as usize;
        let new_path_ptr = arg_i32(&args, 4) as usize;
        let new_path_len = arg_i32(&args, 5) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let memory = memory.borrow();

        let old_path = read_string(&memory, old_path_ptr, old_path_len)?;
        let new_path = read_string(&memory, new_path_ptr, new_path_len)?;
        let old_full = match self.resolve_path_from(_old_dirfd, &old_path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        let new_full = match self.resolve_path_from(_new_dirfd, &new_path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        if fs::rename(old_full, new_full).is_err() {
            return Ok((ERRNO_NOENT as i32).into());
        }

        Ok(0.into())
    }

    fn path_link(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let _old_dirfd = arg_i32(&args, 0) as usize;
        let _old_flags = arg_i32(&args, 1) as u32;
        let old_path_ptr = arg_i32(&args, 2) as usize;
        let old_path_len = arg_i32(&args, 3) as usize;
        let _new_dirfd = arg_i32(&args, 4) as usize;
        let new_path_ptr = arg_i32(&args, 5) as usize;
        let new_path_len = arg_i32(&args, 6) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let memory = memory.borrow();

        let old_path = read_string(&memory, old_path_ptr, old_path_len)?;
        let new_path = read_string(&memory, new_path_ptr, new_path_len)?;
        let old_full = match self.resolve_path_from(_old_dirfd, &old_path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        let new_full = match self.resolve_path_from(_new_dirfd, &new_path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        if fs::hard_link(old_full, new_full).is_err() {
            return Ok((ERRNO_NOENT as i32).into());
        }

        Ok(0.into())
    }

    fn path_symlink(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let old_path_ptr = arg_i32(&args, 0) as usize;
        let old_path_len = arg_i32(&args, 1) as usize;
        let _new_dirfd = arg_i32(&args, 2) as usize;
        let new_path_ptr = arg_i32(&args, 3) as usize;
        let new_path_len = arg_i32(&args, 4) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let memory = memory.borrow();

        let old_path = read_string(&memory, old_path_ptr, old_path_len)?;
        let new_path = read_string(&memory, new_path_ptr, new_path_len)?;
        let new_full = match self.resolve_path_from(_new_dirfd, &new_path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };

        #[cfg(unix)]
        if std::os::unix::fs::symlink(old_path, new_full).is_err() {
            return Ok((ERRNO_NOENT as i32).into());
        }

        Ok(0.into())
    }

    fn path_readlink(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let dirfd = arg_i32(&args, 0) as usize;
        let path_ptr = arg_i32(&args, 1) as usize;
        let path_len = arg_i32(&args, 2) as usize;
        let buf_ptr = arg_i32(&args, 3) as usize;
        let buf_len = arg_i32(&args, 4) as usize;
        let bufused = arg_i32(&args, 5) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let path = read_string(&memory, path_ptr, path_len)?;
        let full = match self.resolve_path_from(dirfd, &path) {
            Ok(path) => path,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        let target = match fs::read_link(full) {
            Ok(target) => target,
            Err(_) => return Ok((ERRNO_NOENT as i32).into()),
        };
        let target_str = target.to_string_lossy();
        let bytes = target_str.as_bytes();
        let len = bytes.len().min(buf_len);
        memory.write_bytes(buf_ptr, &bytes[..len])?;
        write_u32(&mut memory, bufused, len as u32)?;

        Ok(0.into())
    }

    fn clock_res_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let _clock_id = arg_i32(&args, 0);
        let offset = arg_i32(&args, 1) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        write_u64(&mut memory, offset, 1)?;

        Ok(0.into())
    }

    fn clock_time_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let clock_id = arg_i32(&args, 0) as u32;
        let _precision = arg_i64(&args, 1) as u64;
        let offset = arg_i32(&args, 2) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let now = match clock_id {
            CLOCKID_REALTIME => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::from_secs(0))
                .as_nanos() as u64,
            CLOCKID_MONOTONIC => {
                let dur = std::time::Instant::now().elapsed();
                dur.as_nanos() as u64
            }
            _ => 0,
        };

        write_u64(&mut memory, offset, now)?;

        Ok(0.into())
    }

    fn poll_oneoff(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let in_ptr = arg_i32(&args, 0) as usize;
        let out_ptr = arg_i32(&args, 1) as usize;
        let nsubscriptions = arg_i32(&args, 2) as usize;
        let nevents_ptr = arg_i32(&args, 3) as usize;

        let store = store.borrow();
        let memory = store.memory.first().with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let mut min_timeout: Option<Duration> = None;
        let mut subscriptions = Vec::with_capacity(nsubscriptions);

        for i in 0..nsubscriptions {
            let base = in_ptr + i * 48;
            let userdata = read_u64(&memory, base + 0)?;
            let sub_type = read_u8(&memory, base + 8)?;

            let mut error = ERRNO_SUCCESS;
            if sub_type == SUBSCRIPTION_TYPE_CLOCK {
                let clock_id = read_u32(&memory, base + 16)?;
                let timeout = read_u64(&memory, base + 24)?;
                let _precision = read_u64(&memory, base + 32)?;
                let flags = read_u16(&memory, base + 40)?;

                let duration = if flags & SUBSCRIPTION_CLOCK_ABSTIME != 0 {
                    match clock_id {
                        CLOCKID_REALTIME => {
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_else(|_| Duration::from_secs(0))
                                .as_nanos() as u64;
                            if timeout > now {
                                Duration::from_nanos(timeout - now)
                            } else {
                                Duration::from_nanos(0)
                            }
                        }
                        _ => Duration::from_nanos(0),
                    }
                } else {
                    Duration::from_nanos(timeout)
                };
                min_timeout = Some(match min_timeout {
                    Some(current) => current.min(duration),
                    None => duration,
                });
            } else {
                error = ERRNO_INVAL;
            }

            subscriptions.push((userdata, sub_type, error));
        }

        if let Some(timeout) = min_timeout {
            if timeout > Duration::from_nanos(0) {
                std::thread::sleep(timeout);
            }
        }

        for (idx, (userdata, sub_type, error)) in subscriptions.into_iter().enumerate() {
            let base = out_ptr + idx * 32;
            write_u64(&mut memory, base + 0, userdata)?;
            write_u16(&mut memory, base + 8, error)?;
            write_u8(&mut memory, base + 10, sub_type)?;
            write_u64(&mut memory, base + 16, 0)?;
            write_u16(&mut memory, base + 24, 0)?;
        }

        write_u32(&mut memory, nevents_ptr, nsubscriptions as u32)?;

        Ok(0.into())
    }

    fn resolve_path(&self, input: &str) -> Result<PathBuf> {
        let mut input_path = Path::new(input);
        if input_path.is_absolute() {
            input_path = Path::new(input.trim_start_matches('/'));
        }
        let candidate = self.preopen_root.join(input_path);
        let canon = if candidate.exists() {
            candidate.canonicalize()?
        } else {
            let parent = candidate
                .parent()
                .ok_or_else(|| anyhow::anyhow!("invalid path"))?;
            let parent_canon = parent.canonicalize()?;
            parent_canon.join(candidate.file_name().unwrap())
        };
        if !canon.starts_with(&self.preopen_root_canon) {
            bail!("path escape detected");
        }
        Ok(canon)
    }

    fn resolve_path_from(&self, dirfd: usize, input: &str) -> Result<PathBuf> {
        let mut input_path = Path::new(input);
        if input_path.is_absolute() {
            input_path = Path::new(input.trim_start_matches('/'));
        }
        let base = if let Some(file) = self.file_arc(dirfd) {
            let file = file.lock().expect("cannot lock file");
            file.preopen_path().cloned().unwrap_or_else(|| self.preopen_root.clone())
        } else {
            self.preopen_root.clone()
        };

        let candidate = base.join(input_path);
        let canon = if candidate.exists() {
            candidate.canonicalize()?
        } else {
            let parent = candidate
                .parent()
                .ok_or_else(|| anyhow::anyhow!("invalid path"))?;
            let parent_canon = parent.canonicalize()?;
            parent_canon.join(candidate.file_name().unwrap())
        };

        if !canon.starts_with(&self.preopen_root_canon) {
            bail!("path escape detected");
        }
        Ok(canon)
    }
}

fn arg_i32(args: &[Value], idx: usize) -> i32 {
    match &args[idx] {
        Value::I32(v) => *v,
        Value::I64(v) => *v as i32,
        _ => 0,
    }
}

fn arg_i64(args: &[Value], idx: usize) -> i64 {
    match &args[idx] {
        Value::I64(v) => *v,
        Value::I32(v) => *v as i64,
        _ => 0,
    }
}

fn write_u8(memory: &mut crate::execution::module::InternalMemoryInst, offset: usize, value: u8) -> Result<()> {
    memory.write(0, &MemoryArg { align: 1, offset: offset as u32 }, value)
}

fn write_u16(memory: &mut crate::execution::module::InternalMemoryInst, offset: usize, value: u16) -> Result<()> {
    memory.write(0, &MemoryArg { align: 2, offset: offset as u32 }, value)
}

fn write_u32(memory: &mut crate::execution::module::InternalMemoryInst, offset: usize, value: u32) -> Result<()> {
    memory.write(0, &MemoryArg { align: 4, offset: offset as u32 }, value)
}

fn write_u64(memory: &mut crate::execution::module::InternalMemoryInst, offset: usize, value: u64) -> Result<()> {
    memory.write(0, &MemoryArg { align: 8, offset: offset as u32 }, value)
}

fn read_u8(memory: &crate::execution::module::InternalMemoryInst, offset: usize) -> Result<u8> {
    memory.load(0, &MemoryArg { align: 1, offset: offset as u32 })
}

fn read_u16(memory: &crate::execution::module::InternalMemoryInst, offset: usize) -> Result<u16> {
    memory.load(0, &MemoryArg { align: 2, offset: offset as u32 })
}

fn read_u32(memory: &crate::execution::module::InternalMemoryInst, offset: usize) -> Result<u32> {
    memory.load(0, &MemoryArg { align: 4, offset: offset as u32 })
}

fn read_u64(memory: &crate::execution::module::InternalMemoryInst, offset: usize) -> Result<u64> {
    memory.load(0, &MemoryArg { align: 8, offset: offset as u32 })
}

fn read_string(memory: &crate::execution::module::InternalMemoryInst, ptr: usize, len: usize) -> Result<String> {
    let bytes = &memory.data[ptr..ptr + len];
    Ok(String::from_utf8_lossy(bytes).to_string())
}

fn filetype_to_u8(file_type: &fs::FileType) -> u8 {
    if file_type.is_file() {
        4
    } else if file_type.is_dir() {
        3
    } else if file_type.is_symlink() {
        7
    } else {
        0
    }
}

fn time_to_nanos(time: Option<SystemTime>) -> u64 {
    time.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::{
        wasi::{file::FileEntry, wasi_snapshot_preview1::virtual_file::VirtualFile},
        Runtime,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_fd_write() -> Result<()> {
        let code = r#"
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32))
  )
  (memory 1)
  (data (i32.const 0) "Hello, World!\n")

  (func $hello_world (result i32)
    (local $iovec i32)

    (i32.store (i32.const 16) (i32.const 0))
    (i32.store (i32.const 20) (i32.const 14))

    (local.set $iovec (i32.const 16))

    (call $fd_write
      (i32.const 1)
      (local.get $iovec)
      (i32.const 1)
      (i32.const 24)
    )
  )
  (export "_start" (func $hello_world))
)
            "#;
        let wasm = wat::parse_str(code)?;

        let stdin = Arc::new(Mutex::new(FileEntry::new(
            Box::<VirtualFile>::default(),
            FileCaps::Sync,
        )));
        let stdout = Arc::new(Mutex::new(FileEntry::new(
            Box::<VirtualFile>::default(),
            FileCaps::Sync,
        )));

        let wasi = WasiSnapshotPreview1::with_io(vec![stdin, stdout.clone()]);
        let mut runtime = Runtime::from_bytes(wasm.as_slice(), Some(vec![Box::new(wasi)]))?;

        let result: i32 = runtime
            .call("_start".into(), vec![])?
            .expect("not found result")
            .into();
        assert_eq!(result, 0);

        let mut stdout = stdout.lock().expect("cannot lock stdout");
        let stdout = stdout.capbable(FileCaps::Seek)?;
        stdout.seek(SeekFrom::Start(0))?;
        assert_eq!(stdout.read_string()?, "Hello, World!\n");
        Ok(())
    }

    #[test]
    fn test_args_get() -> Result<()> {
        let wasm = wat::parse_file("examples/args_get.wasm")?;

        let stdin = Arc::new(Mutex::new(FileEntry::new(
            Box::<VirtualFile>::default(),
            FileCaps::Sync,
        )));
        let stdout = Arc::new(Mutex::new(FileEntry::new(
            Box::<VirtualFile>::default(),
            FileCaps::Sync,
        )));

        let wasi = WasiSnapshotPreview1::with_io(vec![stdin, stdout.clone()]);
        let mut runtime = Runtime::from_bytes(wasm.as_slice(), Some(vec![Box::new(wasi)]))?;

        runtime.call("_start".into(), vec![])?;

        let mut stdout = stdout.lock().expect("cannot lock stdout");
        let stdout = stdout.capbable(FileCaps::Read)?;
        stdout.seek(SeekFrom::Start(0))?;
        let result: Vec<String> = serde_json::from_str(&stdout.read_string()?)?;
        let arg = std::env::args().take(1).next().unwrap();
        assert_eq!(result[0], arg);
        Ok(())
    }

    #[test]
    fn test_fd_read() -> Result<()> {
        let wasm = wat::parse_file("examples/fd_read.wasm")?;

        let stdin = Arc::new(Mutex::new(FileEntry::new(
            Box::new(VirtualFile::new(b"hello world")),
            FileCaps::Sync,
        )));

        let stdout = Arc::new(Mutex::new(FileEntry::new(
            Box::<VirtualFile>::default(),
            FileCaps::Sync,
        )));

        let wasi = WasiSnapshotPreview1::with_io(vec![stdin.clone(), stdout.clone()]);
        let mut runtime = Runtime::from_bytes(wasm.as_slice(), Some(vec![Box::new(wasi)]))?;

        runtime.call("_start".into(), vec![])?;

        let mut stdout = stdout.lock().expect("cannot lock stdout");
        let stdout = stdout.capbable(FileCaps::Read)?;
        stdout.seek(SeekFrom::Start(0))?;
        assert_eq!(stdout.read_string()?, "input: got: hello world\n");
        Ok(())
    }
}

use super::file::{File, FileTable};
use crate::{
    binary::instruction::MemoryArg, memory_load, memory_write, module::ExternalFuncInst, Importer,
    Store, Value,
};
use anyhow::{Context as _, Result};
use rand::prelude::*;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct WasiSnapshotPreview1 {
    file_table: FileTable,
}

impl Importer for WasiSnapshotPreview1 {
    fn invoke(
        &self,
        store: Rc<RefCell<Store>>,
        func: ExternalFuncInst,
        args: Vec<Value>,
    ) -> Result<Option<Value>> {
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
            _ => todo!(),
        }?;
        Ok(Some(value))
    }
}

impl WasiSnapshotPreview1 {
    pub fn with_io(files: Vec<Arc<Mutex<File>>>) -> Self {
        let file_table = FileTable::with_io(files);
        Self { file_table }
    }

    fn proc_exit(&self, args: Vec<Value>) -> ! {
        let exit_code: i32 = args
            .get(0)
            .expect("no any argument in proc_exit")
            .clone()
            .into();
        std::process::exit(exit_code);
    }

    fn environ_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (mut offset, mut buf_offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.get(0).with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let env = std::env::vars();
        for (key, val) in env {
            memory_write!(memory, 0, 4, offset, buf_offset);
            offset += 4;

            let data = format!("{}={}\0", key, val);
            let data = data.as_bytes();

            // write bytes to memory
            memory.write_bytes(buf_offset, data)?;
            buf_offset += data.len();
        }

        Ok(0.into())
    }

    fn environ_sizes_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (offset, buf_offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.get(0).with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let env = std::env::vars();

        let (size, _) = env.size_hint();
        memory_write!(memory, 0, 4, offset, size);

        let size = env.fold(0, |acc, (key, val)| {
            let data = format!("{}={}\0", key, val);
            let data = data.as_bytes();
            acc + data.len()
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
        let memory = store.memory.get(0).with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let file = self
            .file_table
            .get(fd)
            .with_context(|| format!("cannot get file with fd: {}", fd))?;
        let file = Arc::clone(file);

        let mut nread = 0;
        for _ in 0..iovs_len {
            let offset: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let len: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let offset = offset as usize;
            let end = offset + len as usize;

            nread += file
                .lock()
                .expect("cannot get file lock")
                .read(&mut memory.data[offset..end])?;
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
        let memory = store.memory.get(0).with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let file = self
            .file_table
            .get(fd)
            .with_context(|| format!("cannot get file with fd: {}", fd))?;
        let file = Arc::clone(file);
        let mut written = 0;

        for _ in 0..iovs_len {
            let offset: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let len: i32 = memory_load!(memory, 0, 4, iovs);
            iovs += 4;

            let offset = offset as usize;
            let end = offset + len as usize;
            let buf = &memory.data[offset..end];

            written += file.lock().expect("cannot get file lock").write(buf)?;
        }

        memory_write!(memory, 0, 4, rp, written);

        Ok(0.into())
    }

    fn args_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (mut offset, mut buf_offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.get(0).with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let args = std::env::args();
        for arg in args {
            memory_write!(memory, 0, 4, offset, buf_offset);
            offset += 4;

            let data = format!("{}\0", arg);
            let data = data.as_bytes();

            // write bytes to memory
            memory.write_bytes(buf_offset, data)?;
            buf_offset += data.len();
        }

        Ok(0.into())
    }

    fn args_sizes_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (offset, buf_offset) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.get(0).with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let args = std::env::args();

        let (size, _) = args.size_hint();
        memory_write!(memory, 0, 4, offset, size);

        let size = args.fold(0, |acc, arg| {
            let data = format!("{}\0", arg);
            let data = data.as_bytes();
            acc + data.len()
        });

        memory_write!(memory, 0, 4, buf_offset, size);

        Ok(0.into())
    }

    fn random_get(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Value> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (mut offset, buf_len) = (args[0] as usize, args[1] as usize);

        let store = store.borrow();
        let memory = store.memory.get(0).with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let mut rng = thread_rng();

        let distr = rand::distributions::Uniform::new_inclusive(1u32, 100);
        for _ in 0..buf_len {
            let x = rng.sample(distr);
            let mut buf = std::io::Cursor::new(Vec::new());
            leb128::write::unsigned(&mut buf, x as u64)?;
            memory.write_bytes(offset, buf.into_inner().as_slice())?;
            offset += 1;
        }

        Ok(0.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Runtime;
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

        let stdin = Arc::new(Mutex::new(File::from_buffer(vec![])));
        let stdout = Arc::new(Mutex::new(File::from_buffer(vec![])));

        let wasi = WasiSnapshotPreview1::with_io(vec![stdin, Arc::clone(&stdout)]);
        let mut runtime = Runtime::from_bytes(wasm.as_slice(), Some(Box::new(wasi)))?;

        let result: i32 = runtime
            .call("_start".into(), vec![])?
            .expect("not found result")
            .into();
        assert_eq!(result, 0);

        let mut stdout = stdout.lock().expect("cannot lock stdout");
        stdout.seek(0)?; // NOTE: need to reset cursor for reading
        assert_eq!(stdout.read_string()?, "Hello, World!\n");
        Ok(())
    }

    #[test]
    fn test_args_get() -> Result<()> {
        let wasm = wat::parse_file("examples/args_get.wasm")?;

        let stdin = Arc::new(Mutex::new(File::from_buffer(vec![])));
        let stdout = Arc::new(Mutex::new(File::from_buffer(vec![])));

        let wasi = WasiSnapshotPreview1::with_io(vec![stdin, Arc::clone(&stdout)]);
        let mut runtime = Runtime::from_bytes(wasm.as_slice(), Some(Box::new(wasi)))?;

        runtime.call("_start".into(), vec![])?;

        let mut stdout = stdout.lock().expect("cannot lock stdout");
        stdout.seek(0)?;
        let result: Vec<String> = serde_json::from_str(&stdout.read_string()?)?;
        let arg = std::env::args().take(1).next().unwrap();
        assert_eq!(result[0], arg);
        Ok(())
    }

    #[test]
    fn test_fd_read() -> Result<()> {
        let wasm = wat::parse_file("examples/fd_read.wasm")?;

        let stdin = Arc::new(Mutex::new(File::from_buffer(
            "hello world".as_bytes().to_vec(),
        )));
        let stdout = Arc::new(Mutex::new(File::from_buffer(vec![])));

        let wasi = WasiSnapshotPreview1::with_io(vec![Arc::clone(&stdin), Arc::clone(&stdout)]);
        let mut runtime = Runtime::from_bytes(wasm.as_slice(), Some(Box::new(wasi)))?;

        runtime.call("_start".into(), vec![])?;

        let mut stdout = stdout.lock().expect("cannot lock stdout");
        stdout.seek(0)?;
        assert_eq!(stdout.read_string()?, "input: got: hello world\n");
        Ok(())
    }
}

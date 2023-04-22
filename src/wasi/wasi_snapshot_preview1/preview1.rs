use super::file::{File, FileTable};
use crate::{binary::instruction::MemoryArg, module::ExternalFuncInst, Importer, Store, Value};
use anyhow::{Context as _, Result};
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
        match func.field.as_str() {
            "fd_write" => self.fd_write(store, args),
            _ => todo!(),
        }
    }
}

impl WasiSnapshotPreview1 {
    pub fn with_io(files: Vec<Arc<Mutex<File>>>) -> Self {
        let file_table = FileTable::with_io(files);
        Self { file_table }
    }

    fn fd_write(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Option<Value>> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (fd, iovs, iovs_len, rp) = (
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
            let offset: i32 = memory.load(
                0,
                &MemoryArg {
                    align: 4,
                    offset: iovs as u32,
                },
            )?;

            let len: i32 = memory.load(
                0,
                &MemoryArg {
                    align: 4,
                    offset: iovs as u32 + 4,
                },
            )?;

            let offset = offset as usize;
            let end = offset + len as usize;
            let buf = &memory.data[offset..end];

            written += file.lock().expect("cannot get file lock").write(buf)?;
        }

        memory.write(
            0,
            &MemoryArg {
                align: 4,
                offset: rp as u32,
            },
            written as i32,
        )?;

        Ok(Some((written as i32).into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Runtime;

    #[test]
    fn test_fd_write() -> Result<()> {
        let code = r#"
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32))
  )
  (memory (export "memory") 1)
  (data (i32.const 0) "Hello, World!\n")

  (func $write_hello_world (result i32)
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
  (export "_start" (func $write_hello_world))
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
        assert_eq!(result, 14);

        let mut stdout = stdout.lock().expect("cannot lock stdout");
        stdout.seek(0)?; // NOTE: need to reset cursor for reading
        assert_eq!(stdout.read_string()?, "Hello, World!\n");
        Ok(())
    }
}

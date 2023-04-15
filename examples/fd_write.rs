use anyhow::Result;
use chibiwasm::wasi::wasi_snapshot_preview1::Wasi;
use chibiwasm::{Runtime, Value};

fn main() -> Result<()> {
    let wasi = Wasi {};
    let mut runtime = Runtime::from_file("examples/fd_write.wasm", Some(Box::new(wasi)))?;

    let fd = Value::I32(1);
    let iovs = Value::I32(0);
    let iovs_len = Value::I32(15);
    let rp = Value::I32(0);

    runtime.call("main".into(), vec![fd, iovs, iovs_len, rp])?;
    Ok(())
}

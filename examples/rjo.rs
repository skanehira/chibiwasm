use anyhow::Result;
use chibiwasm::wasi::WasiSnapshotPreview1;
use chibiwasm::Runtime;

fn main() -> Result<()> {
    let wasi = WasiSnapshotPreview1::default();
    let mut runtime = Runtime::from_file("examples/rjo.wasm", Some(Box::new(wasi)))?;
    runtime.call("_start".into(), vec![])?;
    Ok(())
}

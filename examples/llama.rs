use anyhow::Result;
use chibiwasm::wasi::{WasiEphemeralNn, WasiSnapshotPreview1};
use chibiwasm::Runtime;

fn main() -> Result<()> {
    let wasi = Box::<WasiSnapshotPreview1>::default();
    let wasi_nn = Box::new(WasiEphemeralNn::new(
        "default:GGML:AUTO:llama-2-7b-chat-q5_k_m.gguf",
    ));

    let mut runtime = Runtime::from_file("examples/llama-chat.wasm", Some(vec![wasi, wasi_nn]))?;
    runtime.call("_start".into(), vec![])?;
    Ok(())
}

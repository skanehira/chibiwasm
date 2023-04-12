use chibiwasm::{Runtime, Value};

fn main() -> anyhow::Result<()> {
    let mut runtime = Runtime::from_file("examples/fib.wasm", None)?;
    if let Some(output) = runtime.call("fib".into(), vec![Value::I32(10)])? {
        println!("output: {}", output);
    }
    Ok(())
}

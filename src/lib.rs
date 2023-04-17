//!# chibiwasm
//! This repository was created for the purpose of learning how Wasm works.
//! Please do not use it in production.
//!
//! ## Usage
//! ```sh
//! $ cat
//! (module
//!  (func $add (export "add") (param i32 i32) (result i32)
//!    (local.get 0)
//!    (local.get 1)
//!    (i32.add)
//!  )
//! )
//! $ wat2wasm add.wat
//! $ cargo run -- add.wasm add 1 2
//!    Finished dev [unoptimized + debuginfo] target(s) in 0.09s
//!     Running `target/debug/chibiwasm add.wasm add 1 2`
//! 3
//! ```
//!
//! ## Use as a crate
//! 
//! ```rust
//! use chibiwasm::{Runtime, Value};
//! 
//! fn main() -> anyhow::Result<()> {
//!    let mut runtime = Runtime::from_file("examples/fib.wasm", None)?;
//!    if let Some(output) = runtime.call("fib".into(), vec![Value::I32(10)])? {
//!        println!("output: {}", output);
//!    }
//!    Ok(())
//!}
//!```
//!
//!```sh
//!$ cargo run -q --example fib
//!output: 89
//!```



mod binary;
pub mod execution;
pub use execution::*;

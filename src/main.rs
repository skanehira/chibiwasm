use anyhow::Result;

use chibiwasm::module::Decoder;
use chibiwasm::runtime::Runtime;
use chibiwasm::value::Value;
use clap::Parser;
use std::fs;

#[derive(Debug, Parser)]
#[clap(author, about, version)]
struct Args {
    file: String,

    func: String,

    func_args: Vec<i32>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let file = fs::File::open(args.file)?;
    let mut decoder = Decoder::new(file);
    let mut module = decoder.decode()?;
    let mut runtime = Runtime::new(&mut module)?;
    let mut func_args = vec![];
    for arg in args.func_args.into_iter() {
        func_args.push(Value::from(arg));
    }
    let result = runtime.invoke(args.func, &mut func_args);
    println!("{}", result?.unwrap());
    Ok(())
}

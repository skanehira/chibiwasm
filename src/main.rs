use anyhow::Result;
use chibiwasm::execution::value::Value;
use chibiwasm::execution::runtime::Runtime;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(author, about, version)]
struct Args {
    file: String,

    func: String,

    func_args: Vec<i32>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut runtime = Runtime::from_file(&args.file)?;
    let func_args: Vec<Value> = args.func_args.into_iter().map(Value::from).collect();
    let result = runtime.call(args.func, func_args)?;
    println!("{:?}", result);
    Ok(())
}

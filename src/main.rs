use anyhow::Result;
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
    let func_args = args.func_args.into_iter().map(Into::into).collect();

    let mut runtime = Runtime::from_file(&args.file)?;
    let result = runtime.call(args.func, func_args)?;

    match result {
        Some(result) => {
            println!("{:?}", result);
        }
        _ => {}
    }
    Ok(())
}

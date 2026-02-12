use anyhow::Result;
use chibiwasm::{execution::Runtime, wasi::WasiSnapshotPreview1};
use clap::ArgAction;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(author, about, version)]
struct Args {
    file: String,
    func: String,
    func_args: Option<Vec<i32>>,
    #[clap(long = "wasi-arg", action = ArgAction::Append)]
    wasi_args: Vec<String>,
}

fn main() -> Result<()> {
    pretty_env_logger::init();

    let Args {
        file,
        func,
        func_args,
        wasi_args,
    } = Args::parse();

    let args = match func_args {
        Some(args) => args.into_iter().map(Into::into).collect(),
        None => {
            vec![]
        }
    };

    let mut wasi = WasiSnapshotPreview1::default();
    if !wasi_args.is_empty() {
        let mut args = Vec::with_capacity(wasi_args.len() + 1);
        args.push("ruby".to_string());
        args.extend(wasi_args);
        wasi.set_args_env(args, std::env::vars().collect());
    }
    let mut runtime = Runtime::from_file(&file, Some(vec![Box::new(wasi)]))?;
    let result = match runtime.call(func, args) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("runtime error: {err}");
            return Err(err);
        }
    };

    if let Some(output) = result {
        println!("{}", output);
    }
    Ok(())
}

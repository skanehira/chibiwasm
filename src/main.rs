#![allow(dead_code)]
#![allow(unused)]

use crate::value::Value;
use anyhow::Result;
use anyhow::{bail, Context};
use clap::Parser;
use module::Module;
use runtime::Runtime;
use section::*;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::{env, result};
use types::FuncType;

mod instruction;
mod module;
mod runtime;
mod section;
mod types;
mod value;

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
    let mut decoder = module::Decoder::new(file);
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

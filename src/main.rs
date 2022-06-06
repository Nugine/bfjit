mod bfir;
mod bfjit;
mod error;

use crate::bfjit::BfVM;

use std::io::{stdin, stdout};
use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, clap::Parser)]
#[clap(version)]
struct Opt {
    #[clap(name = "FILE")]
    file_path: PathBuf,

    #[clap(short = 'o', long = "optimize", help = "Optimize code")]
    optimize: bool,
}

fn main() {
    let opt = Opt::parse();

    let stdin = stdin();
    let stdout = stdout();

    let ret = BfVM::new(
        &opt.file_path,
        Box::new(stdin.lock()),
        Box::new(stdout.lock()),
        opt.optimize,
    )
    .and_then(|mut vm| vm.run());

    if let Err(e) = &ret {
        eprintln!("bfjit: {}", e);
    }

    std::process::exit(ret.is_err() as i32)
}

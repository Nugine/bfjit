mod bfir;
mod bfjit;
#[cfg(feature = "llvm")]
mod bfjit_llvm;
mod error;

use crate::bfjit::BfVM;
#[cfg(feature = "llvm")]
use crate::bfjit_llvm::BfLlvmVM;

use std::io::{stdin, stdout};
use std::path::PathBuf;

use clap::Parser;
#[cfg(feature = "llvm")]
use clap::ValueEnum;

#[cfg(feature = "llvm")]
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Backend {
    Dynasm,
    Llvm,
}

#[derive(Debug, clap::Parser)]
#[clap(version)]
struct Opt {
    #[clap(name = "FILE")]
    file_path: PathBuf,

    #[clap(short = 'o', long = "optimize", help = "Optimize code")]
    optimize: bool,

    #[cfg(feature = "llvm")]
    #[clap(long, value_enum, default_value_t = Backend::Dynasm, help = "JIT backend")]
    backend: Backend,
}

fn main() {
    let opt = Opt::parse();

    let stdin = stdin();
    let stdout = stdout();

    #[cfg(not(feature = "llvm"))]
    let ret = BfVM::new(
        &opt.file_path,
        Box::new(stdin.lock()),
        Box::new(stdout.lock()),
        opt.optimize,
    )
    .and_then(|mut vm| vm.run());

    #[cfg(feature = "llvm")]
    let ret = match opt.backend {
        Backend::Dynasm => BfVM::new(
            &opt.file_path,
            Box::new(stdin.lock()),
            Box::new(stdout.lock()),
            opt.optimize,
        )
        .and_then(|mut vm| vm.run()),
        Backend::Llvm => BfLlvmVM::new(
            &opt.file_path,
            Box::new(stdin.lock()),
            Box::new(stdout.lock()),
            opt.optimize,
        )
        .and_then(|mut vm| vm.run()),
    };

    if let Err(e) = &ret {
        eprintln!("bfjit: {e}");
    }

    std::process::exit(ret.is_err() as i32)
}

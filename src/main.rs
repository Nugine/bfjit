#![feature(proc_macro_hygiene)]

mod bfir;
mod bfjit;
mod error;

use crate::bfjit::BfVM;
use std::io::{stdin, stdout, Read, Write};
use std::path::Path;

fn main() {
    let stdin = stdin();
    let stdout = stdout();
    let path = std::env::args().nth(1).unwrap();
    let mut vm = BfVM::new(
        Path::new(&path),
        Box::new(stdin.lock()),
        Box::new(stdout.lock()),
        true
    )
    .unwrap();

    vm.run().unwrap();
}

#![feature(proc_macro_hygiene)]

mod bfir;
mod error;

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};

use std::io::{stdout, Write};

unsafe extern "sysv64" fn print(buf: *const u8, len: u64) -> u8 {
    let ret = std::panic::catch_unwind(|| {
        let buf = std::slice::from_raw_parts(buf, len as usize);
        stdout().write_all(buf).is_err()
    });
    match ret {
        Ok(false) => 0,
        Ok(true) => 1,
        Err(_) => 2,
    }
}

fn main() {
    let mut ops = dynasmrt::x64::Assembler::new().unwrap();
    let string = b"Hello, JIT!\n";

    dynasm!(ops
        ; .arch x64
        ; ->hello:
        ; .bytes string
    );

    let hello = ops.offset();
    dynasm!(ops
        ; lea rdi, [->hello]
        ; mov rsi, QWORD string.len() as _
        ; mov rax, QWORD print as _
        ; call rax
        ; ret
    );

    let buf = ops.finalize().unwrap();

    let hello_fn: unsafe extern "sysv64" fn() -> u8 =
        unsafe { std::mem::transmute(buf.ptr(hello)) };

    let ret = unsafe { hello_fn() };

    assert_eq!(ret, 0);
}

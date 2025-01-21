use crate::bfir::{self, BfIR};
use crate::error::{Result, RuntimeError, VMError};

use std::io::{Read, Write};
use std::path::Path;
use std::ptr;

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};

const MEMORY_SIZE: usize = 4 * 1024 * 1024;

pub struct BfVM<'io> {
    code: dynasmrt::ExecutableBuffer,
    start: dynasmrt::AssemblyOffset,
    memory: Box<[u8]>,
    input: Box<dyn Read + 'io>,
    output: Box<dyn Write + 'io>,
}

#[inline(always)]
fn vm_error(re: RuntimeError) -> *mut VMError {
    let e = Box::new(VMError::from(re));
    Box::into_raw(e)
}

impl BfVM<'_> {
    unsafe extern "sysv64" fn getbyte(this: *mut Self, ptr: *mut u8) -> *mut VMError {
        let mut buf = [0_u8];
        let this = &mut *this;
        match this.input.read(&mut buf) {
            Ok(0) => {}
            Ok(1) => *ptr = buf[0],
            Err(e) => return vm_error(RuntimeError::IO(e)),
            _ => unreachable!(),
        }
        ptr::null_mut()
    }

    unsafe extern "sysv64" fn putbyte(this: *mut Self, ptr: *const u8) -> *mut VMError {
        let buf = std::slice::from_ref(&*ptr);
        let this = &mut *this;
        match this.output.write_all(buf) {
            Ok(()) => ptr::null_mut(),
            Err(e) => vm_error(RuntimeError::IO(e)),
        }
    }

    unsafe extern "sysv64" fn overflow_error() -> *mut VMError {
        vm_error(RuntimeError::PointerOverflow)
    }
}

impl<'io> BfVM<'io> {
    pub fn new(
        file_path: &Path,
        input: Box<dyn Read + 'io>,
        output: Box<dyn Write + 'io>,
        optimize: bool,
    ) -> Result<Self> {
        let src = std::fs::read_to_string(file_path)?;
        let mut ir = bfir::compile(&src)?;
        drop(src);

        if optimize {
            bfir::optimize(&mut ir);
        }
        let (code, start) = Self::compile(&ir)?;
        drop(ir);

        let memory = vec![0; MEMORY_SIZE].into_boxed_slice();
        Ok(Self {
            code,
            start,
            memory,
            input,
            output,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        type RawFn = unsafe extern "sysv64" fn(
            this: *mut BfVM<'_>,
            memory_start: *mut u8,
            memory_end: *const u8,
        ) -> *mut VMError;

        let raw_fn: RawFn = unsafe { std::mem::transmute(self.code.ptr(self.start)) };

        let this: *mut Self = self;
        let memory_start = self.memory.as_mut_ptr();
        let memory_end = unsafe { memory_start.add(MEMORY_SIZE) };

        let ret: *mut VMError = unsafe { raw_fn(this, memory_start, memory_end) };

        if ret.is_null() {
            Ok(())
        } else {
            Err(*unsafe { Box::from_raw(ret) })
        }
    }
}

impl BfVM<'_> {
    #[allow(clippy::fn_to_numeric_cast)]
    fn compile(code: &[BfIR]) -> Result<(dynasmrt::ExecutableBuffer, dynasmrt::AssemblyOffset)> {
        let mut ops = dynasmrt::x64::Assembler::new()?;
        let start = ops.offset();

        let mut loops = vec![];

        // this:         rdi r12
        // memory_start: rsi r13
        // memory_end:   rdx r14
        // ptr:              r15

        dynasm!(ops
            ; push rax
            ; push r12
            ; push r13
            ; push r14
            ; push r15
            ; mov r12, rdi   // save this
            ; mov r13, rsi   // save memory_start
            ; mov r14, rdx   // save memory_end
            ; mov r15, rsi   // ptr = memory_start
        );

        use BfIR::*;
        for &ir in code {
            match ir {
                AddPtr(x) => dynasm!(ops
                    ; add r15, x as i32     // ptr += x
                    ; jc  ->overflow        // jmp if overflow
                    ; cmp r15, r14          // ptr - memory_end
                    ; jnb ->overflow        // jmp if ptr >= memory_end
                ),
                SubPtr(x) => dynasm!(ops
                    ; sub r15, x as i32     // ptr -= x
                    ; jc  ->overflow        // jmp if overflow
                    ; cmp r15, r13          // ptr - memory_start
                    ; jb  ->overflow        // jmp if ptr < memory_start
                ),
                AddVal(x) => dynasm!(ops
                    ; add BYTE [r15], x as i8    // *ptr += x
                ),
                SubVal(x) => dynasm!(ops
                    ; sub BYTE [r15], x as i8    // *ptr -= x
                ),
                GetByte => dynasm!(ops
                    ; mov  rdi, r12         // arg0: this
                    ; mov  rsi, r15         // arg1: ptr
                    ; mov  rax, QWORD BfVM::getbyte as _
                    ; call rax              // getbyte(this, ptr)
                    ; test rax, rax
                    ; jnz  ->io_error       // jmp if rax != 0
                ),
                PutByte => dynasm!(ops
                    ; mov  rdi, r12         // arg0: this
                    ; mov  rsi, r15         // arg1: ptr
                    ; mov  rax, QWORD BfVM::putbyte as _
                    ; call rax              // putbyte(this, ptr)
                    ; test rax, rax
                    ; jnz  ->io_error       // jmp if rax != 0
                ),
                Jz => {
                    let left = ops.new_dynamic_label();
                    let right = ops.new_dynamic_label();
                    loops.push((left, right));

                    dynasm!(ops
                        ; cmp BYTE [r15], 0
                        ; jz => right       // jmp if *ptr == 0
                        ; => left
                    )
                }
                Jnz => {
                    let (left, right) = loops.pop().unwrap();
                    dynasm!(ops
                        ; cmp BYTE [r15], 0
                        ; jnz => left       // jmp if *ptr != 0
                        ; => right
                    )
                }
            }
        }

        dynasm!(ops
            ; xor rax, rax
            ; jmp >exit
            ; -> overflow:
            ; mov rax, QWORD BfVM::overflow_error as _
            ; call rax
            ; jmp >exit
            ; -> io_error:
            ; exit:
            ; pop r15
            ; pop r14
            ; pop r13
            ; pop r12
            ; pop rdx
            ; ret
        );

        let code = ops.finalize().unwrap();

        Ok((code, start))
    }
}

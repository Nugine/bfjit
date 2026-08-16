use crate::bfir::{self, BfIR};
use crate::error::{Result, RuntimeError, VMError};

use std::io::{Read, Write};
use std::path::Path;
use std::ptr;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::Module;
use inkwell::types::{IntType, PointerType};
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};

const MEMORY_SIZE: usize = 4 * 1024 * 1024;

pub struct BfLlvmVM<'io> {
    _context: &'static Context,
    _engine: ExecutionEngine<'static>,
    start: usize,
    memory: Box<[u8]>,
    input: Box<dyn Read + 'io>,
    output: Box<dyn Write + 'io>,
}

#[inline(always)]
fn vm_error(re: RuntimeError) -> *mut VMError {
    let e = Box::new(VMError::from(re));
    Box::into_raw(e)
}

#[inline]
fn llvm_error<E: std::fmt::Display>(e: E) -> VMError {
    VMError::Llvm(e.to_string())
}

impl BfLlvmVM<'_> {
    unsafe extern "C" fn getbyte(this: *mut Self, ptr: *mut u8) -> *mut VMError {
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

    unsafe extern "C" fn putbyte(this: *mut Self, ptr: *const u8) -> *mut VMError {
        let buf = std::slice::from_ref(&*ptr);
        let this = &mut *this;
        match this.output.write_all(buf) {
            Ok(()) => ptr::null_mut(),
            Err(e) => vm_error(RuntimeError::IO(e)),
        }
    }

    unsafe extern "C" fn overflow_error() -> *mut VMError {
        vm_error(RuntimeError::PointerOverflow)
    }
}

impl<'io> BfLlvmVM<'io> {
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

        Self::from_ir(&ir, input, output)
    }

    fn from_ir(
        code: &[BfIR],
        input: Box<dyn Read + 'io>,
        output: Box<dyn Write + 'io>,
    ) -> Result<Self> {
        let (context, engine, start) = Self::compile(code)?;
        let memory = vec![0; MEMORY_SIZE].into_boxed_slice();
        Ok(Self {
            _context: context,
            _engine: engine,
            start,
            memory,
            input,
            output,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        type RawFn = unsafe extern "C" fn(
            this: *mut BfLlvmVM<'_>,
            memory_start: *mut u8,
            memory_end: *const u8,
        ) -> *mut VMError;

        let raw_fn: RawFn = unsafe { std::mem::transmute(self.start) };

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

impl BfLlvmVM<'_> {
    #[allow(clippy::type_complexity)]
    fn compile(code: &[BfIR]) -> Result<(&'static Context, ExecutionEngine<'static>, usize)> {
        let context = Box::leak(Box::new(Context::create()));
        let module = context.create_module("bfjit_llvm");
        let builder = context.create_builder();

        let i8_type = context.i8_type();
        let usize_type = context.i64_type();
        let ptr_type = context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false);
        let function = module.add_function("bfjit_main", fn_type, None);

        let callback_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        let getbyte = module.add_function("bfjit_llvm_getbyte", callback_type, None);
        let putbyte = module.add_function("bfjit_llvm_putbyte", callback_type, None);
        let overflow_error = module.add_function(
            "bfjit_llvm_overflow_error",
            ptr_type.fn_type(&[], false),
            None,
        );

        Self::build_function(
            code,
            context,
            &module,
            &builder,
            function,
            getbyte,
            putbyte,
            overflow_error,
            ptr_type,
            i8_type,
            usize_type,
        )?;

        module.verify().map_err(llvm_error)?;

        let engine = module
            .create_jit_execution_engine(OptimizationLevel::Default)
            .map_err(llvm_error)?;

        engine.add_global_mapping(&getbyte, Self::getbyte as *const () as usize);
        engine.add_global_mapping(&putbyte, Self::putbyte as *const () as usize);
        engine.add_global_mapping(&overflow_error, Self::overflow_error as *const () as usize);

        let start = engine
            .get_function_address("bfjit_main")
            .map_err(llvm_error)?;

        Ok((context, engine, start))
    }

    #[allow(clippy::too_many_arguments)]
    fn build_function<'ctx>(
        code: &[BfIR],
        context: &'ctx Context,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        function: FunctionValue<'ctx>,
        getbyte: FunctionValue<'ctx>,
        putbyte: FunctionValue<'ctx>,
        overflow_error: FunctionValue<'ctx>,
        ptr_type: PointerType<'ctx>,
        i8_type: IntType<'ctx>,
        usize_type: IntType<'ctx>,
    ) -> Result<()> {
        let entry = context.append_basic_block(function, "entry");
        let overflow = context.append_basic_block(function, "overflow");

        builder.position_at_end(entry);

        let this = function.get_nth_param(0).unwrap().into_pointer_value();
        let memory_start = function.get_nth_param(1).unwrap().into_pointer_value();
        let memory_end = function.get_nth_param(2).unwrap().into_pointer_value();

        let ptr_slot = builder.build_alloca(ptr_type, "ptr").map_err(llvm_error)?;
        builder
            .build_store(ptr_slot, memory_start)
            .map_err(llvm_error)?;

        use BfIR::*;
        let mut loops: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>)> = vec![];

        for &ir in code {
            match ir {
                AddPtr(x) => Self::emit_move_ptr(
                    context,
                    builder,
                    ptr_slot,
                    ptr_type,
                    usize_type,
                    memory_start,
                    memory_end,
                    overflow,
                    i64::from(x),
                    IntPredicate::UGE,
                )?,
                SubPtr(x) => Self::emit_move_ptr(
                    context,
                    builder,
                    ptr_slot,
                    ptr_type,
                    usize_type,
                    memory_start,
                    memory_end,
                    overflow,
                    -i64::from(x),
                    IntPredicate::ULT,
                )?,
                AddVal(x) => Self::emit_update_cell(builder, ptr_slot, ptr_type, i8_type, x, true)?,
                SubVal(x) => {
                    Self::emit_update_cell(builder, ptr_slot, ptr_type, i8_type, x, false)?
                }
                GetByte => Self::emit_io_call(context, builder, this, ptr_slot, ptr_type, getbyte)?,
                PutByte => Self::emit_io_call(context, builder, this, ptr_slot, ptr_type, putbyte)?,
                Jz => {
                    let body = context.append_basic_block(function, "loop.body");
                    let after = context.append_basic_block(function, "loop.after");
                    let cell = Self::load_cell(builder, ptr_slot, ptr_type, i8_type)?;
                    let zero = i8_type.const_zero();
                    let is_zero = builder
                        .build_int_compare(IntPredicate::EQ, cell, zero, "loop.is_zero")
                        .map_err(llvm_error)?;
                    builder
                        .build_conditional_branch(is_zero, after, body)
                        .map_err(llvm_error)?;
                    builder.position_at_end(body);
                    loops.push((body, after));
                }
                Jnz => {
                    let (body, after) = loops.pop().unwrap();
                    let cell = Self::load_cell(builder, ptr_slot, ptr_type, i8_type)?;
                    let zero = i8_type.const_zero();
                    let is_not_zero = builder
                        .build_int_compare(IntPredicate::NE, cell, zero, "loop.is_not_zero")
                        .map_err(llvm_error)?;
                    builder
                        .build_conditional_branch(is_not_zero, body, after)
                        .map_err(llvm_error)?;
                    builder.position_at_end(after);
                }
            }
        }

        builder
            .build_return(Some(&ptr_type.const_null()))
            .map_err(llvm_error)?;

        builder.position_at_end(overflow);
        let err = builder
            .build_call(overflow_error, &[], "overflow_error")
            .map_err(llvm_error)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        builder.build_return(Some(&err)).map_err(llvm_error)?;

        module.verify().map_err(llvm_error)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_move_ptr<'ctx>(
        context: &'ctx Context,
        builder: &Builder<'ctx>,
        ptr_slot: PointerValue<'ctx>,
        ptr_type: PointerType<'ctx>,
        usize_type: IntType<'ctx>,
        memory_start: PointerValue<'ctx>,
        memory_end: PointerValue<'ctx>,
        overflow: BasicBlock<'ctx>,
        offset: i64,
        bounds_predicate: IntPredicate,
    ) -> Result<()> {
        let function = builder
            .get_insert_block()
            .and_then(|block| block.get_parent())
            .unwrap();
        let ok = context.append_basic_block(function, "ptr.ok");

        let ptr = Self::load_ptr(builder, ptr_slot, ptr_type)?;
        let ptr_int = builder
            .build_ptr_to_int(ptr, usize_type, "ptr.int")
            .map_err(llvm_error)?;
        let offset_int = usize_type.const_int(offset as u64, true);
        let next_int = if offset >= 0 {
            builder
                .build_int_add(ptr_int, offset_int, "ptr.next_int")
                .map_err(llvm_error)?
        } else {
            builder
                .build_int_sub(
                    ptr_int,
                    usize_type.const_int(offset.unsigned_abs(), false),
                    "ptr.next_int",
                )
                .map_err(llvm_error)?
        };

        let wrapped = if offset >= 0 {
            builder
                .build_int_compare(IntPredicate::ULT, next_int, ptr_int, "ptr.wrapped")
                .map_err(llvm_error)?
        } else {
            builder
                .build_int_compare(IntPredicate::UGT, next_int, ptr_int, "ptr.wrapped")
                .map_err(llvm_error)?
        };

        let bound = match bounds_predicate {
            IntPredicate::UGE => memory_end,
            IntPredicate::ULT => memory_start,
            _ => unreachable!(),
        };
        let bound_int = builder
            .build_ptr_to_int(bound, usize_type, "ptr.bound")
            .map_err(llvm_error)?;
        let out_of_bounds = builder
            .build_int_compare(bounds_predicate, next_int, bound_int, "ptr.out_of_bounds")
            .map_err(llvm_error)?;
        let failed = builder
            .build_or(wrapped, out_of_bounds, "ptr.failed")
            .map_err(llvm_error)?;

        builder
            .build_conditional_branch(failed, overflow, ok)
            .map_err(llvm_error)?;

        builder.position_at_end(ok);
        let next_ptr = builder
            .build_int_to_ptr(next_int, ptr_type, "ptr.next")
            .map_err(llvm_error)?;
        builder
            .build_store(ptr_slot, next_ptr)
            .map_err(llvm_error)?;

        Ok(())
    }

    fn emit_update_cell<'ctx>(
        builder: &Builder<'ctx>,
        ptr_slot: PointerValue<'ctx>,
        ptr_type: PointerType<'ctx>,
        i8_type: IntType<'ctx>,
        value: u8,
        add: bool,
    ) -> Result<()> {
        let ptr = Self::load_ptr(builder, ptr_slot, ptr_type)?;
        let cell = builder
            .build_load(i8_type, ptr, "cell")
            .map_err(llvm_error)?
            .into_int_value();
        let value = i8_type.const_int(u64::from(value), false);
        let next = if add {
            builder.build_int_add(cell, value, "cell.next")
        } else {
            builder.build_int_sub(cell, value, "cell.next")
        }
        .map_err(llvm_error)?;
        builder.build_store(ptr, next).map_err(llvm_error)?;

        Ok(())
    }

    fn emit_io_call<'ctx>(
        context: &'ctx Context,
        builder: &Builder<'ctx>,
        this: PointerValue<'ctx>,
        ptr_slot: PointerValue<'ctx>,
        ptr_type: PointerType<'ctx>,
        callback: FunctionValue<'ctx>,
    ) -> Result<()> {
        let function = builder
            .get_insert_block()
            .and_then(|block| block.get_parent())
            .unwrap();
        let io_error = context.append_basic_block(function, "io.error");
        let io_ok = context.append_basic_block(function, "io.ok");

        let ptr = Self::load_ptr(builder, ptr_slot, ptr_type)?;
        let err = builder
            .build_call(callback, &[this.into(), ptr.into()], "io")
            .map_err(llvm_error)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let is_error = builder
            .build_int_compare(
                IntPredicate::NE,
                builder
                    .build_ptr_to_int(err, context.i64_type(), "io.err_int")
                    .map_err(llvm_error)?,
                context.i64_type().const_zero(),
                "io.is_error",
            )
            .map_err(llvm_error)?;
        builder
            .build_conditional_branch(is_error, io_error, io_ok)
            .map_err(llvm_error)?;

        builder.position_at_end(io_error);
        builder.build_return(Some(&err)).map_err(llvm_error)?;

        builder.position_at_end(io_ok);

        Ok(())
    }

    fn load_ptr<'ctx>(
        builder: &Builder<'ctx>,
        ptr_slot: PointerValue<'ctx>,
        ptr_type: PointerType<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        Ok(builder
            .build_load(ptr_type, ptr_slot, "ptr")
            .map_err(llvm_error)?
            .into_pointer_value())
    }

    fn load_cell<'ctx>(
        builder: &Builder<'ctx>,
        ptr_slot: PointerValue<'ctx>,
        ptr_type: PointerType<'ctx>,
        i8_type: IntType<'ctx>,
    ) -> Result<IntValue<'ctx>> {
        let ptr = Self::load_ptr(builder, ptr_slot, ptr_type)?;
        Ok(builder
            .build_load(i8_type, ptr, "cell")
            .map_err(llvm_error)?
            .into_int_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::RefCell;
    use std::io::{self, Write};
    use std::rc::Rc;

    struct SharedOutput(Rc<RefCell<Vec<u8>>>);

    impl Write for SharedOutput {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_llvm_vm_runs_ir() {
        let code = bfir::compile("+.+.").unwrap();
        let output = Rc::new(RefCell::new(Vec::new()));
        let writer = SharedOutput(Rc::clone(&output));
        let mut vm = BfLlvmVM::from_ir(&code, Box::new(io::empty()), Box::new(writer)).unwrap();

        vm.run().unwrap();

        assert_eq!(&*output.borrow(), &[1, 2]);
    }
}

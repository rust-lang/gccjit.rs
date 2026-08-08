use std::ffi::CString;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::c_int;
use std::ptr::{self, NonNull};

use asm::ExtendedAsm;
use block;
use context::{Case, Context};
use function::{self, Function};
use location::{self, Location};
use lvalue::{self, ToLValue};
use object::{self, Object, ToObject};
#[cfg(feature = "master")]
use region::{self, Region};
use rvalue::{self, ToRValue};

use crate::{with_lib, with_lib_handle, with_lib_without_error_check};

/// BinaryOp is a enum representing the various binary operations
/// that gccjit knows how to codegen.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
    Plus,
    Minus,
    Mult,
    Divide,
    Modulo,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    LogicalAnd,
    LogicalOr,
    LShift,
    RShift,
}

/// UnaryOp is an enum representing the various unary operations
/// that gccjit knows how to codegen.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum UnaryOp {
    Minus,
    BitwiseNegate,
    LogicalNegate,
    Abs,
}

/// ComparisonOp is an enum representing the various comparisons that
/// gccjit is capable of doing.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub enum ComparisonOp {
    Equals,
    NotEquals,
    LessThan,
    LessThanEquals,
    GreaterThan,
    GreaterThanEquals,
}

/// Block represents a basic block in gccjit. Blocks are created by functions.
/// A basic block consists of a series of instructions terminated by a terminator
/// instruction, which can be either a jump to one block, a conditional branch to
/// two blocks (true/false branches), a return, or a void return.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Block<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_block>,
}

impl<'ctx> ToObject<'ctx> for Block<'ctx> {
    fn to_object(&self) -> Object<'ctx> {
        with_lib_without_error_check(|lib| unsafe {
            let ptr = lib.gcc_jit_block_as_object(get_ptr(self));
            object::from_ptr(ptr).expect("Failed to get Object from Block")
        })
    }
}

impl<'ctx> crate::ContextGetter<'ctx> for Block<'ctx> {
    fn context(&self) -> crate::ContextRef<'ctx> {
        self.to_object().context()
    }
}

impl<'ctx> fmt::Debug for Block<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        let obj = self.to_object();
        obj.fmt(fmt)
    }
}

impl<'ctx> Block<'ctx> {
    pub fn get_function(&self) -> Option<Function<'ctx>> {
        with_lib(self, |lib| unsafe {
            let ptr = lib.gcc_jit_block_get_function(get_ptr(self));
            function::from_ptr(ptr)
        })
    }

    #[cfg(feature = "master")]
    #[track_caller]
    pub fn get_successors(&self) -> Vec<Block<'ctx>> {
        with_lib_handle(self, |lib| unsafe {
            let count = lib.gcc_jit_block_get_successor_count(get_ptr(self));
            (0..count)
                .map(|index| from_ptr(lib.gcc_jit_block_get_successor(get_ptr(self), index)))
                .collect::<Option<Vec<_>>>()
        })
    }

    /// Evaluates the rvalue parameter and discards its result. Equivalent
    /// to (void)<expr> in C.
    pub fn add_eval<T: ToRValue<'ctx>>(&self, loc: Option<Location<'ctx>>, value: T) {
        let rvalue = value.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_add_eval(get_ptr(self), loc_ptr, rvalue::get_ptr(&rvalue));
        });
    }

    #[cfg(feature = "master")]
    pub fn add_try_catch(
        &self,
        loc: Option<Location<'ctx>>,
        try_region: Region<'ctx>,
        catch_region: Region<'ctx>,
    ) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_add_try_catch(
                get_ptr(self),
                loc_ptr,
                region::get_ptr(&try_region),
                region::get_ptr(&catch_region),
            );
        });
    }

    #[cfg(feature = "master")]
    pub fn add_try_finally(
        &self,
        loc: Option<Location<'ctx>>,
        try_region: Region<'ctx>,
        finally_region: Region<'ctx>,
    ) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_add_try_finally(
                get_ptr(self),
                loc_ptr,
                region::get_ptr(&try_region),
                region::get_ptr(&finally_region),
            );
        });
    }

    #[cfg(feature = "master")]
    pub fn add_cleanup(
        &self,
        loc: Option<Location<'ctx>>,
        try_region: Region<'ctx>,
        cleanup_region: Region<'ctx>,
    ) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_add_cleanup(
                get_ptr(self),
                loc_ptr,
                region::get_ptr(&try_region),
                region::get_ptr(&cleanup_region),
            );
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    /// Assigns the value of an rvalue to an lvalue directly. Equivalent
    /// to <lvalue> = <rvalue> in C.
    pub fn add_assignment<L: ToLValue<'ctx>, R: ToRValue<'ctx>>(
        &self,
        loc: Option<Location<'ctx>>,
        assign_target: L,
        value: R,
    ) {
        let lvalue = assign_target.to_lvalue();
        let rvalue = value.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_add_assignment(
                get_ptr(self),
                loc_ptr,
                lvalue::get_ptr(&lvalue),
                rvalue::get_ptr(&rvalue),
            );
        });
    }

    /// Performs a binary operation on an LValue and an RValue, assigning
    /// the result of the binary operation to the LValue upon completion.
    /// Equivalent to the *=, +=, -=, etc. operator family in C.
    pub fn add_assignment_op<L: ToLValue<'ctx>, R: ToRValue<'ctx>>(
        &self,
        loc: Option<Location<'ctx>>,
        assign_target: L,
        op: BinaryOp,
        value: R,
    ) {
        let lvalue = assign_target.to_lvalue();
        let rvalue = value.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_add_assignment_op(
                get_ptr(self),
                loc_ptr,
                lvalue::get_ptr(&lvalue),
                mem::transmute::<BinaryOp, gccjit_sys::gcc_jit_binary_op>(op),
                rvalue::get_ptr(&rvalue),
            );
        });
    }

    /// Adds a comment to a block. It's unclear from the documentation what
    /// this actually means.
    pub fn add_comment<S: AsRef<str>>(&self, loc: Option<Location<'ctx>>, message: S) {
        let message_ref = message.as_ref();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            let cstr = CString::new(message_ref).unwrap();
            lib.gcc_jit_block_add_comment(get_ptr(self), loc_ptr, cstr.as_ptr());
        });
    }

    /// Terminates a block by branching to one of two blocks, depending
    /// on the value of a conditional RValue.
    pub fn end_with_conditional<T: ToRValue<'ctx>>(
        &self,
        loc: Option<Location<'ctx>>,
        cond: T,
        on_true: Block<'ctx>,
        on_false: Block<'ctx>,
    ) {
        let cond_rvalue = cond.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_end_with_conditional(
                get_ptr(self),
                loc_ptr,
                rvalue::get_ptr(&cond_rvalue),
                get_ptr(&on_true),
                get_ptr(&on_false),
            );
        });
    }

    /// Terminates a block by unconditionally jumping to another block.
    pub fn end_with_jump(&self, loc: Option<Location<'ctx>>, target: Block<'ctx>) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_end_with_jump(get_ptr(self), loc_ptr, get_ptr(&target));
        });
    }

    /// Terminates a block by returning from the containing function, setting
    /// the rvalue to be the return value of the function. This is equivalent
    /// to C's "return <expr>". This function can only be used to terminate
    /// a block within a function whose return type is not void.
    pub fn end_with_return<T: ToRValue<'ctx>>(&self, loc: Option<Location<'ctx>>, ret: T) {
        let ret_rvalue = ret.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_end_with_return(get_ptr(self), loc_ptr, rvalue::get_ptr(&ret_rvalue));
        });
    }

    /// Terminates a block by returning from the containing function, returning
    /// no value. This is equivalent to C's bare "return" with no expression.
    /// This function can only be used to terminate a block within a function
    /// that returns void.
    pub fn end_with_void_return(&self, loc: Option<Location<'ctx>>) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_end_with_void_return(get_ptr(self), loc_ptr);
        });
    }

    pub fn end_with_switch<T: ToRValue<'ctx>>(
        &self,
        loc: Option<Location<'ctx>>,
        expr: T,
        default_block: Block<'ctx>,
        cases: &[Case<'ctx>],
    ) {
        let expr = expr.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_end_with_switch(
                get_ptr(self),
                loc_ptr,
                rvalue::get_ptr(&expr),
                block::get_ptr(&default_block),
                cases.len() as c_int,
                cases.as_ptr() as *mut *mut _,
            );
        });
    }

    #[cfg(feature = "master")]
    pub fn end_with_fallthrough(&self, loc: Option<Location<'ctx>>) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_block_end_with_fallthrough(get_ptr(self), loc_ptr);
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    #[track_caller]
    pub fn add_extended_asm(
        &self,
        loc: Option<Location<'ctx>>,
        asm_template: &str,
    ) -> ExtendedAsm<'ctx> {
        let asm_template = CString::new(asm_template).unwrap();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib_handle(self, |lib| unsafe {
            ExtendedAsm::from_ptr(lib.gcc_jit_block_add_extended_asm(
                get_ptr(self),
                loc_ptr,
                asm_template.as_ptr(),
            ))
        })
    }

    #[track_caller]
    pub fn end_with_extended_asm_goto(
        &self,
        loc: Option<Location<'ctx>>,
        asm_template: &str,
        goto_blocks: &[Block<'ctx>],
        fallthrough_block: Option<Block<'ctx>>,
    ) -> ExtendedAsm<'ctx> {
        let asm_template = CString::new(asm_template).unwrap();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        let fallthrough_block_ptr = match fallthrough_block {
            Some(ref block) => unsafe { get_ptr(block) },
            None => ptr::null_mut(),
        };
        with_lib_handle(self, |lib| unsafe {
            ExtendedAsm::from_ptr(lib.gcc_jit_block_end_with_extended_asm_goto(
                get_ptr(self),
                loc_ptr,
                asm_template.as_ptr(),
                goto_blocks.len() as c_int,
                goto_blocks.as_ptr() as *mut _,
                fallthrough_block_ptr,
            ))
        })
    }
}

#[cfg(feature = "master")]
#[track_caller]
pub fn clone_blocks<'ctx>(blocks: &[Block<'ctx>]) -> Vec<Block<'ctx>> {
    if blocks.is_empty() {
        return Vec::new();
    }
    with_lib_handle(&blocks[0], |lib| {
        let mut src: Vec<_> = blocks
            .iter()
            .map(|block| unsafe { get_ptr(block) })
            .collect();
        let mut dst: Vec<*mut gccjit_sys::gcc_jit_block> = vec![ptr::null_mut(); blocks.len()];
        unsafe {
            lib.gcc_jit_blocks_clone(src.len() as c_int, src.as_mut_ptr(), dst.as_mut_ptr());
            dst.into_iter()
                .map(|ptr| from_ptr(ptr))
                .collect::<Option<Vec<_>>>()
        }
    })
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_block) -> Option<Block<'ctx>> {
    Some(Block {
        marker: PhantomData,
        ptr: NonNull::new(ptr)?,
    })
}

pub unsafe fn get_ptr<'ctx>(block: &Block<'ctx>) -> *mut gccjit_sys::gcc_jit_block {
    block.ptr.as_ptr()
}

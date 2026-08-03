use std::marker::PhantomData;
use std::ffi::CString;
use std::fmt;
use std::ptr;
use std::mem;
use std::os::raw::c_int;

use asm::ExtendedAsm;
use block;
use context::{Case, Context};
use object::{self, ToObject, Object};
use function::{self, Function};
use location::{self, Location};
use region::{self, Region};
use rvalue::{self, ToRValue};
use lvalue::{self, ToLValue};

use crate::with_lib;

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
    RShift
}

/// UnaryOp is an enum representing the various unary operations
/// that gccjit knows how to codegen.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum UnaryOp {
    Minus,
    BitwiseNegate,
    LogicalNegate,
    Abs
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
    GreaterThanEquals
}

/// Block represents a basic block in gccjit. Blocks are created by functions.
/// A basic block consists of a series of instructions terminated by a terminator
/// instruction, which can be either a jump to one block, a conditional branch to
/// two blocks (true/false branches), a return, or a void return.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Block<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: *mut gccjit_sys::gcc_jit_block,
}

impl<'ctx> ToObject<'ctx> for Block<'ctx> {
    fn to_object(&self) -> Object<'ctx> {
        with_lib(|lib| {
            unsafe {
                let ptr = lib.gcc_jit_block_as_object(self.ptr);
                object::from_ptr(ptr)
            }
        })
    }
}

impl<'ctx> fmt::Debug for Block<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        let obj = self.to_object();
        obj.fmt(fmt)
    }
}

impl<'ctx> Block<'ctx> {
    pub fn get_function(&self) -> Function<'ctx> {
        with_lib(|lib| {
            unsafe {
                let ptr = lib.gcc_jit_block_get_function(self.ptr);
                function::from_ptr(ptr)
            }
        })
    }

    /// Returns the blocks control can transfer to from the end of this block,
    /// i.e. the blocks its terminating statement branches to. A block that has
    /// not been terminated transfers control nowhere, and thus has no
    /// successors. The order is unspecified, and a block can occur more than
    /// once (a conditional both of whose edges lead to the same block has two
    /// successors).
    pub fn get_successors(&self) -> Vec<Block<'ctx>> {
        with_lib(|lib| {
            unsafe {
                let count = lib.gcc_jit_block_get_successor_count(self.ptr);
                (0..count)
                    .map(|index| from_ptr(lib.gcc_jit_block_get_successor(self.ptr, index)))
                    .collect()
            }
        })
    }

    /// Evaluates the rvalue parameter and discards its result. Equivalent
    /// to (void)<expr> in C.
    pub fn add_eval<T: ToRValue<'ctx>>(&self,
                                       loc: Option<Location<'ctx>>,
                                       value: T) {
        let rvalue = value.to_rvalue();
        let loc_ptr = match loc {
                Some(loc) => unsafe { location::get_ptr(&loc) },
                None => ptr::null_mut()
            };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_add_eval(self.ptr, loc_ptr, rvalue::get_ptr(&rvalue));
            }
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    #[cfg(feature="master")]
    pub fn add_try_catch(&self, loc: Option<Location<'ctx>>, try_region: Region<'ctx>, catch_region: Region<'ctx>) {
        let loc_ptr = match loc {
                Some(loc) => unsafe { location::get_ptr(&loc) },
                None => ptr::null_mut()
            };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_add_try_catch(self.ptr, loc_ptr, region::get_ptr(&try_region), region::get_ptr(&catch_region));
            }
        });
    }

    #[cfg(feature="master")]
    pub fn add_try_finally(&self, loc: Option<Location<'ctx>>, try_region: Region<'ctx>, finally_region: Region<'ctx>) {
        let loc_ptr = match loc {
                Some(loc) => unsafe { location::get_ptr(&loc) },
                None => ptr::null_mut()
            };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_add_try_finally(self.ptr, loc_ptr, region::get_ptr(&try_region), region::get_ptr(&finally_region));
            }
        });
    }

    /// Adds a cleanup construct: the `cleanup_region` runs only on the
    /// unwind path out of `try_region`, and then unwinding resumes (the
    /// middle-end synthesizes the appropriate context-sensitive resume).
    /// Both regions may span several blocks; a block in a region that
    /// resumes unwinding (the cleanup's exit) is terminated with
    /// `Block::end_with_fallthrough`.
    #[cfg(feature="master")]
    pub fn add_cleanup(&self, loc: Option<Location<'ctx>>, try_region: Region<'ctx>, cleanup_region: Region<'ctx>) {
        let loc_ptr = match loc {
                Some(loc) => unsafe { location::get_ptr(&loc) },
                None => ptr::null_mut()
            };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_add_cleanup(self.ptr, loc_ptr,
                    region::get_ptr(&try_region), region::get_ptr(&cleanup_region));
            }
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    /// Assigns the value of an rvalue to an lvalue directly. Equivalent
    /// to <lvalue> = <rvalue> in C.
    pub fn add_assignment<L: ToLValue<'ctx>, R: ToRValue<'ctx>>(&self,
                                                                loc: Option<Location<'ctx>>,
                                                                assign_target: L,
                                                                value: R) {
        let lvalue = assign_target.to_lvalue();
        let rvalue = value.to_rvalue();
        let loc_ptr = match loc {
                Some(loc) => unsafe { location::get_ptr(&loc) },
                None => ptr::null_mut()
            };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_add_assignment(self.ptr, loc_ptr, lvalue::get_ptr(&lvalue),
                    rvalue::get_ptr(&rvalue));
            }
        });

        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    /// Performs a binary operation on an LValue and an RValue, assigning
    /// the result of the binary operation to the LValue upon completion.
    /// Equivalent to the *=, +=, -=, etc. operator family in C.
    pub fn add_assignment_op<L: ToLValue<'ctx>, R: ToRValue<'ctx>>(&self,
                                                                   loc: Option<Location<'ctx>>,
                                                                   assign_target: L,
                                                                   op: BinaryOp,
                                                                   value: R) {
        let lvalue = assign_target.to_lvalue();
        let rvalue = value.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut()
        };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_add_assignment_op(self.ptr, loc_ptr, lvalue::get_ptr(&lvalue),
                    mem::transmute::<BinaryOp, gccjit_sys::gcc_jit_binary_op>(op), rvalue::get_ptr(&rvalue));
            }
        });
    }

    /// Adds a comment to a block. It's unclear from the documentation what
    /// this actually means.
    pub fn add_comment<S: AsRef<str>>(&self,
                       loc: Option<Location<'ctx>>,
                       message: S) {
        let message_ref = message.as_ref();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut()
        };
        with_lib(|lib| {
            unsafe {
                let cstr = CString::new(message_ref).unwrap();
                lib.gcc_jit_block_add_comment(self.ptr, loc_ptr, cstr.as_ptr());
            }
        });
    }

    /// Terminates a block by branching to one of two blocks, depending
    /// on the value of a conditional RValue.
    pub fn end_with_conditional<T: ToRValue<'ctx>>(&self,
                                loc: Option<Location<'ctx>>,
                                cond: T,
                                on_true: Block<'ctx>,
                                on_false: Block<'ctx>) {
        let cond_rvalue = cond.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut()
        };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_end_with_conditional(self.ptr, loc_ptr, rvalue::get_ptr(&cond_rvalue),
                    on_true.ptr, on_false.ptr);
            }
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    /// Terminates a block by unconditionally jumping to another block.
    pub fn end_with_jump(&self,
                         loc: Option<Location<'ctx>>,
                         target: Block<'ctx>) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut()
        };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_end_with_jump(self.ptr, loc_ptr, target.ptr);
            }
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    /// Terminates a block by returning from the containing function, setting
    /// the rvalue to be the return value of the function. This is equivalent
    /// to C's "return <expr>". This function can only be used to terminate
    /// a block within a function whose return type is not void.
    pub fn end_with_return<T: ToRValue<'ctx>>(&self,
                                              loc: Option<Location<'ctx>>,
                                              ret: T) {
        let ret_rvalue = ret.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut()
        };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_end_with_return(self.ptr, loc_ptr, rvalue::get_ptr(&ret_rvalue));
            }
        });

        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    /// Terminates a block by returning from the containing function, returning
    /// no value. This is equivalent to C's bare "return" with no expression.
    /// This function can only be used to terminate a block within a function
    /// that returns void.
    pub fn end_with_void_return(&self, loc: Option<Location<'ctx>>) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut()
        };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_end_with_void_return(self.ptr, loc_ptr);
            }
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    /// Terminates a block by falling through to the end of its enclosing
    /// structured construct instead of by an explicit jump, return or
    /// resume. This is intended for the exceptional (cleanup) body of a
    /// try/finally created with `add_try_finally`: leaving that body by
    /// fall-through lets the middle-end synthesize the context-sensitive
    /// resume (RESX) rather than an unconditional cross-frame
    /// `_Unwind_Resume`. A block terminated this way validates as
    /// terminated and reports no successors.
    pub fn end_with_fallthrough(&self, loc: Option<Location<'ctx>>) {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut()
        };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_end_with_fallthrough(self.ptr, loc_ptr);
            }
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    pub fn end_with_switch<T: ToRValue<'ctx>>(&self, loc: Option<Location<'ctx>>, expr: T, default_block: Block<'ctx>, cases: &[Case<'ctx>]) {
        let expr = expr.to_rvalue();
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut()
        };
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_block_end_with_switch(self.ptr, loc_ptr, rvalue::get_ptr(&expr), block::get_ptr(&default_block),
                cases.len() as c_int, cases.as_ptr() as *mut *mut _);
            }
        });
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = self.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
    }

    pub fn add_extended_asm(&self, loc: Option<Location<'ctx>>, asm_template: &str) -> ExtendedAsm<'ctx> {
        let asm_template = CString::new(asm_template).unwrap();
        let loc_ptr =
            match loc {
                Some(loc) => unsafe { location::get_ptr(&loc) },
                None => ptr::null_mut(),
            };
        with_lib(|lib| {
            unsafe {
                ExtendedAsm::from_ptr(lib.gcc_jit_block_add_extended_asm(self.ptr, loc_ptr, asm_template.as_ptr()))
            }
        })
    }

    pub fn end_with_extended_asm_goto(&self, loc: Option<Location<'ctx>>, asm_template: &str, goto_blocks: &[Block<'ctx>], fallthrough_block: Option<Block<'ctx>>) -> ExtendedAsm<'ctx> {
        let asm_template = CString::new(asm_template).unwrap();
        let loc_ptr =
            match loc {
                Some(loc) => unsafe { location::get_ptr(&loc) },
                None => ptr::null_mut(),
            };
        let fallthrough_block_ptr =
            match fallthrough_block {
                Some(ref block) => unsafe { get_ptr(block) },
                None => ptr::null_mut(),
            };
        with_lib(|lib| {
            unsafe {
                ExtendedAsm::from_ptr(lib.gcc_jit_block_end_with_extended_asm_goto(self.ptr, loc_ptr, asm_template.as_ptr(), goto_blocks.len() as c_int, goto_blocks.as_ptr() as *mut _, fallthrough_block_ptr))
            }
        })
    }
}

/// Clones the given blocks (and, transitively, any blocks they structurally
/// inline, such as a try/catch's try and handler blocks). Returns the clone of
/// each input block, in the same order (so the first returned clone
/// corresponds to the first input block). References among the cloned set are
/// remapped to the clones; references to blocks outside the set are preserved.
/// The originals are left untouched. The clones are loose blocks: they are not
/// placed in any region, so the caller decides where they go (e.g. adopts them
/// into a region with `Region::add_block`). All blocks must belong to the same
/// function.
pub fn clone_blocks<'ctx>(blocks: &[Block<'ctx>]) -> Vec<Block<'ctx>> {
    if blocks.is_empty() {
        return Vec::new();
    }
    with_lib(|lib| {
        let mut src: Vec<_> = blocks.iter().map(|block| unsafe { get_ptr(block) }).collect();
        let mut dst: Vec<*mut gccjit_sys::gcc_jit_block> = vec![ptr::null_mut(); blocks.len()];
        unsafe {
            lib.gcc_jit_blocks_clone(src.len() as c_int, src.as_mut_ptr(), dst.as_mut_ptr());
            dst.into_iter().map(|ptr| from_ptr(ptr)).collect()
        }
    })
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_block) -> Block<'ctx> {
    Block {
        marker: PhantomData,
        ptr
    }
}

pub unsafe fn get_ptr<'ctx>(block: &Block<'ctx>) -> *mut gccjit_sys::gcc_jit_block {
    block.ptr
}

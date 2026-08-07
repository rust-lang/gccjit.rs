use std::ffi::CString;
use std::fmt;
use std::marker::PhantomData;

use block::{self, Block};
use context::Context;

use crate::with_lib;

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Region<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: *mut gccjit_sys::gcc_jit_region,
}

impl<'ctx> fmt::Debug for Region<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        write!(fmt, "Region ({:?})", self.ptr)
    }
}

impl<'ctx> Region<'ctx> {
    pub fn new_block<S: AsRef<str>>(&self, name: S) -> Block<'ctx> {
        with_lib(|lib| unsafe {
            let cstr = CString::new(name.as_ref()).unwrap();
            let ptr = lib.gcc_jit_region_new_block(self.ptr, cstr.as_ptr());
            block::from_ptr(ptr)
        })
    }

    pub fn add_block(&self, blk: Block<'ctx>) {
        with_lib(|lib| unsafe {
            lib.gcc_jit_region_add_block(self.ptr, block::get_ptr(&blk));
        })
    }
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_region) -> Region<'ctx> {
    Region {
        marker: PhantomData,
        ptr,
    }
}

pub unsafe fn get_ptr<'ctx>(region: &Region<'ctx>) -> *mut gccjit_sys::gcc_jit_region {
    region.ptr
}

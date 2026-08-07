use std::ffi::CString;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

use block::{self, Block};
use context::Context;

use crate::{with_lib, with_lib_without_error_check};

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Region<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_region>,
}

impl<'ctx> fmt::Debug for Region<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        unsafe { write!(fmt, "Region ({:?})", get_ptr(self)) }
    }
}

impl<'ctx> Region<'ctx> {
    pub fn new_block<S: AsRef<str>>(&self, name: S) -> Option<Block<'ctx>> {
        with_lib_without_error_check(|lib| unsafe {
            let cstr = CString::new(name.as_ref()).unwrap();
            let ptr = lib.gcc_jit_region_new_block(get_ptr(self), cstr.as_ptr());
            block::from_ptr(ptr)
        })
    }

    pub fn add_block(&self, blk: Block<'ctx>) {
        with_lib(&blk, |lib| unsafe {
            lib.gcc_jit_region_add_block(get_ptr(self), block::get_ptr(&blk));
        })
    }
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_region) -> Option<Region<'ctx>> {
    Some(Region {
        marker: PhantomData,
        ptr: NonNull::new(ptr)?,
    })
}

pub unsafe fn get_ptr<'ctx>(region: &Region<'ctx>) -> *mut gccjit_sys::gcc_jit_region {
    region.ptr.as_ptr()
}

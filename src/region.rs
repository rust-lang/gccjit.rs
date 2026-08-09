use std::ffi::CString;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

use block::{self, Block};
use context::Context;

use crate::object::ToObject;
use crate::{with_lib, with_lib_handle, with_lib_without_error_check};

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Region<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_region>,
}

impl<'ctx> crate::ToObject<'ctx> for Region<'ctx> {
    fn to_object(&self) -> crate::Object<'ctx> {
        with_lib_without_error_check(|lib| unsafe {
            let ptr = lib.gcc_jit_region_as_object(get_ptr(self));
            crate::object::from_ptr(ptr).expect("Failed to get Object from Region")
        })
    }
}

impl<'ctx> crate::ContextGetter<'ctx> for Region<'ctx> {
    fn context(&self) -> crate::ContextRef<'ctx> {
        self.to_object().context()
    }
}

impl<'ctx> fmt::Debug for Region<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        let obj = self.to_object();
        obj.fmt(fmt)
    }
}

impl<'ctx> Region<'ctx> {
    #[track_caller]
    pub fn new_block<S: AsRef<str>>(&self, name: S) -> Block<'ctx> {
        with_lib_handle(self, |lib| unsafe {
            let cstr = CString::new(name.as_ref()).unwrap();
            let ptr = lib.gcc_jit_region_new_block(get_ptr(self), cstr.as_ptr());
            block::from_ptr(ptr)
        })
    }

    pub fn add_block(&self, blk: Block<'ctx>) {
        with_lib(self, |lib| unsafe {
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

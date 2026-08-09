use context::Context;
use lvalue;
use lvalue::{LValue, ToLValue};
use object;
use object::{Object, ToObject};
use rvalue;
use rvalue::{RValue, ToRValue};
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

use crate::{with_lib, with_lib_without_error_check};

/// Parameter represents a parameter to a function. A series of parameteres
/// can be combined to form a function signature.
#[derive(Copy, Clone, PartialEq)]
pub struct Parameter<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_param>,
}

impl<'ctx> ToObject<'ctx> for Parameter<'ctx> {
    fn to_object(&self) -> Object<'ctx> {
        with_lib_without_error_check(|lib| unsafe {
            object::from_ptr(lib.gcc_jit_param_as_object(get_ptr(self)))
                .expect("Failed to get Object from Parameter")
        })
    }
}

impl<'ctx> crate::ContextGetter<'ctx> for Parameter<'ctx> {
    fn context(&self) -> crate::ContextRef<'ctx> {
        self.to_object().context()
    }
}

impl<'ctx> fmt::Debug for Parameter<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        let obj = self.to_object();
        obj.fmt(fmt)
    }
}

impl<'ctx> ToRValue<'ctx> for Parameter<'ctx> {
    fn to_rvalue(&self) -> RValue<'ctx> {
        with_lib(self, |lib| unsafe {
            let ptr = lib.gcc_jit_param_as_rvalue(get_ptr(self));
            rvalue::from_ptr(ptr).expect("Failed to convert Parameter to RValue")
        })
    }
}

impl<'ctx> ToLValue<'ctx> for Parameter<'ctx> {
    fn to_lvalue(&self) -> LValue<'ctx> {
        with_lib(self, |lib| unsafe {
            let ptr = lib.gcc_jit_param_as_lvalue(get_ptr(self));
            lvalue::from_ptr(ptr).expect("Failed to convert Parameter to LValue")
        })
    }
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_param) -> Option<Parameter<'ctx>> {
    Some(Parameter {
        marker: PhantomData,
        ptr: NonNull::new(ptr)?,
    })
}

pub unsafe fn get_ptr<'ctx>(loc: &Parameter<'ctx>) -> *mut gccjit_sys::gcc_jit_param {
    loc.ptr.as_ptr()
}

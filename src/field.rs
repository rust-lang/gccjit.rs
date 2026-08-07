use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

use context::Context;
use object;
use object::{Object, ToObject};

use crate::with_lib_without_error_check;

/// Field represents a field that composes structs or unions. A number of fields
/// can be combined to create either a struct or a union.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Field<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_field>,
}

impl<'ctx> ToObject<'ctx> for Field<'ctx> {
    fn to_object(&self) -> Object<'ctx> {
        with_lib_without_error_check(|lib| unsafe {
            object::from_ptr(lib.gcc_jit_field_as_object(get_ptr(self)))
                .expect("Failed to get Object from Field")
        })
    }
}

impl<'ctx> crate::ContextGetter<'ctx> for Field<'ctx> {
    fn context(&self) -> crate::ContextRef<'ctx> {
        self.to_object().context()
    }
}

impl<'ctx> fmt::Debug for Field<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        let obj = self.to_object();
        obj.fmt(fmt)
    }
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_field) -> Option<Field<'ctx>> {
    Some(Field {
        marker: PhantomData,
        ptr: NonNull::new(ptr)?,
    })
}

pub unsafe fn get_ptr<'ctx>(f: &Field<'ctx>) -> *mut gccjit_sys::gcc_jit_field {
    f.ptr.as_ptr()
}

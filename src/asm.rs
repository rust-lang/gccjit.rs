use std::ffi::CString;
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::ptr::NonNull;

use crate::{with_lib, with_lib_without_error_check};

use {lvalue, object, rvalue, Context, LValue, Object, RValue, ToObject};

#[derive(Copy, Clone)]
pub struct ExtendedAsm<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_extended_asm>,
}

impl<'ctx> ToObject<'ctx> for ExtendedAsm<'ctx> {
    fn to_object(&self) -> Object<'ctx> {
        with_lib_without_error_check(|lib| unsafe {
            let ptr = lib.gcc_jit_extended_asm_as_object(get_ptr(self));
            object::from_ptr(ptr).expect("Failed to get Object from ExtendedAsm")
        })
    }
}

impl<'ctx> crate::ContextGetter<'ctx> for ExtendedAsm<'ctx> {
    fn context(&self) -> crate::ContextRef<'ctx> {
        self.to_object().context()
    }
}

impl<'ctx> ExtendedAsm<'ctx> {
    pub fn set_volatile_flag(&self, flag: bool) {
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_extended_asm_set_volatile_flag(get_ptr(self), flag as c_int);
        })
    }

    pub fn set_inline_flag(&self, flag: bool) {
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_extended_asm_set_inline_flag(get_ptr(self), flag as c_int);
        })
    }

    pub fn add_output_operand(
        &self,
        asm_symbolic_name: Option<&str>,
        constraint: &str,
        dest: LValue<'ctx>,
    ) {
        let asm_symbolic_name = asm_symbolic_name.map(|name| CString::new(name).unwrap());
        let asm_symbolic_name = match asm_symbolic_name {
            Some(name) => name.as_ptr(),
            None => std::ptr::null_mut(),
        };
        let constraint = CString::new(constraint).unwrap();
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_extended_asm_add_output_operand(
                get_ptr(self),
                asm_symbolic_name,
                constraint.as_ptr(),
                lvalue::get_ptr(&dest),
            );
        })
    }

    pub fn add_input_operand(
        &self,
        asm_symbolic_name: Option<&str>,
        constraint: &str,
        src: RValue<'ctx>,
    ) {
        let asm_symbolic_name = asm_symbolic_name.map(|name| CString::new(name).unwrap());
        let asm_symbolic_name = match asm_symbolic_name {
            Some(name) => name.as_ptr(),
            None => std::ptr::null_mut(),
        };
        let constraint = CString::new(constraint).unwrap();
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_extended_asm_add_input_operand(
                get_ptr(self),
                asm_symbolic_name,
                constraint.as_ptr(),
                rvalue::get_ptr(&src),
            );
        })
    }

    pub fn add_clobber(&self, victim: &str) {
        let victim = CString::new(victim).unwrap();
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_extended_asm_add_clobber(get_ptr(self), victim.as_ptr());
        })
    }

    pub unsafe fn from_ptr(ptr: *mut gccjit_sys::gcc_jit_extended_asm) -> Option<Self> {
        Some(Self {
            marker: PhantomData,
            ptr: NonNull::new(ptr)?,
        })
    }
}

pub unsafe fn get_ptr<'ctx>(asm: &ExtendedAsm<'ctx>) -> *mut gccjit_sys::gcc_jit_extended_asm {
    asm.ptr.as_ptr()
}

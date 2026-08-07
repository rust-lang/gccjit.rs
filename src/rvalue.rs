use block::BinaryOp;
use context::Context;
use field;
use field::Field;
use location;
use location::Location;
use lvalue;
use lvalue::LValue;
use object;
use object::{Object, ToObject};
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Rem, Shl, Shr, Sub};
use std::ptr::{self, NonNull};
use types;
use types::Type;

use crate::{with_lib, with_lib_without_error_check};

/// An RValue is a value that may or may not have a storage address in gccjit.
/// RValues can be dereferenced, used for field accesses, and are the parameters
/// given to a majority of the gccjit API calls.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct RValue<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_rvalue>,
}

/// ToRValue is a trait implemented by types that can be converted to, or
/// treated as, an RValue.
pub trait ToRValue<'ctx> {
    fn to_rvalue(&self) -> RValue<'ctx>;
}

impl<'ctx> ToObject<'ctx> for RValue<'ctx> {
    fn to_object(&self) -> Object<'ctx> {
        with_lib_without_error_check(|lib| unsafe {
            object::from_ptr(lib.gcc_jit_rvalue_as_object(get_ptr(self)))
                .expect("Failed to get Object from RValue")
        })
    }
}

impl<'ctx> crate::ContextGetter<'ctx> for RValue<'ctx> {
    fn context(&self) -> crate::ContextRef<'ctx> {
        self.to_object().context()
    }
}

impl<'ctx> fmt::Debug for RValue<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        let obj = self.to_object();
        obj.fmt(fmt)
    }
}

impl<'ctx> ToRValue<'ctx> for RValue<'ctx> {
    fn to_rvalue(&self) -> RValue<'ctx> {
        unsafe { from_ptr(get_ptr(self)).expect("Failed to convert RValue to RValue") }
    }
}

macro_rules! binary_operator_for {
    ($ty:ty, $name:ident, $op:expr) => {
        impl<'ctx> $ty for RValue<'ctx> {
            type Output = RValue<'ctx>;

            fn $name(self, rhs: RValue<'ctx>) -> RValue<'ctx> {
                with_lib(&self, |lib| unsafe {
                    let rhs_rvalue = rhs.to_rvalue();
                    let obj_ptr = object::get_ptr(&self.to_object());
                    let ctx_ptr = lib.gcc_jit_object_get_context(obj_ptr);
                    let ty = rhs
                        .get_type()
                        .expect("Failed to get Type to compute RValue");
                    let ptr = lib.gcc_jit_context_new_binary_op(
                        ctx_ptr,
                        ptr::null_mut(),
                        mem::transmute::<BinaryOp, gccjit_sys::gcc_jit_binary_op>($op),
                        types::get_ptr(&ty),
                        get_ptr(&self),
                        rhs_rvalue.ptr.as_ptr(),
                    );
                    from_ptr(ptr).expect("Failed to compute RValue")
                })
            }
        }
    };
}

// Operator overloads for ease of manipulation of rvalues
binary_operator_for!(Add, add, BinaryOp::Plus);
binary_operator_for!(Sub, sub, BinaryOp::Minus);
binary_operator_for!(Mul, mul, BinaryOp::Mult);
binary_operator_for!(Div, div, BinaryOp::Divide);
binary_operator_for!(Rem, rem, BinaryOp::Modulo);
binary_operator_for!(BitAnd, bitand, BinaryOp::BitwiseAnd);
binary_operator_for!(BitOr, bitor, BinaryOp::BitwiseOr);
binary_operator_for!(BitXor, bitxor, BinaryOp::BitwiseXor);
binary_operator_for!(Shl<RValue<'ctx>>, shl, BinaryOp::LShift);
binary_operator_for!(Shr<RValue<'ctx>>, shr, BinaryOp::RShift);

impl<'ctx> RValue<'ctx> {
    /// Gets the type of this RValue.
    pub fn get_type(&self) -> Option<Type<'ctx>> {
        with_lib(self, |lib| unsafe {
            let ptr = lib.gcc_jit_rvalue_get_type(get_ptr(self));
            types::from_ptr(ptr)
        })
    }

    /// Sets the location of this RValue.
    #[cfg(feature = "master")]
    pub fn set_location(&self, loc: Location) {
        with_lib(self, |lib| unsafe {
            let loc_ptr = location::get_ptr(&loc);
            lib.gcc_jit_rvalue_set_location(get_ptr(self), loc_ptr);
        })
    }

    /// Change the type of this RValue.
    #[cfg(feature = "master")]
    pub fn set_type(&self, typ: Type<'ctx>) {
        with_lib(self, |lib| unsafe {
            let type_ptr = types::get_ptr(&typ);
            lib.gcc_jit_rvalue_set_type(get_ptr(self), type_ptr);
        })
    }

    /// Given an RValue x and a Field f, returns an RValue representing
    /// C's x.f.
    pub fn access_field(
        &self,
        loc: Option<Location<'ctx>>,
        field: Field<'ctx>,
    ) -> Option<RValue<'ctx>> {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            let ptr =
                lib.gcc_jit_rvalue_access_field(get_ptr(self), loc_ptr, field::get_ptr(&field));
            from_ptr(ptr)
        })
    }

    /// Given an RValue x and a Field f, returns an LValue representing
    /// C's x->f.
    pub fn dereference_field(
        &self,
        loc: Option<Location<'ctx>>,
        field: Field<'ctx>,
    ) -> Option<LValue<'ctx>> {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            let ptr = lib.gcc_jit_rvalue_dereference_field(
                get_ptr(self),
                loc_ptr,
                field::get_ptr(&field),
            );
            lvalue::from_ptr(ptr)
        })
    }

    /// Given a RValue x, returns an RValue that represents *x.
    pub fn dereference(&self, loc: Option<Location<'ctx>>) -> Option<LValue<'ctx>> {
        let loc_ptr = match loc {
            Some(loc) => unsafe { location::get_ptr(&loc) },
            None => ptr::null_mut(),
        };
        with_lib(self, |lib| unsafe {
            let ptr = lib.gcc_jit_rvalue_dereference(get_ptr(self), loc_ptr);
            lvalue::from_ptr(ptr)
        })
    }

    pub fn set_require_tail_call(&self, require_tail_call: bool) {
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_rvalue_set_bool_require_tail_call(get_ptr(self), require_tail_call as _);
        })
    }
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_rvalue) -> Option<RValue<'ctx>> {
    Some(RValue {
        marker: PhantomData,
        ptr: NonNull::new(ptr)?,
    })
}

pub unsafe fn get_ptr<'ctx>(rvalue: &RValue<'ctx>) -> *mut gccjit_sys::gcc_jit_rvalue {
    rvalue.ptr.as_ptr()
}

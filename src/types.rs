use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

use context;
use context::Context;
use object;
use object::{Object, ToObject};
use structs::{self, Struct};

use gccjit_sys::gcc_jit_types::*;

#[cfg(feature = "master")]
use crate::lvalue::AttributeValue;
use crate::{with_lib, with_lib_without_error_check};

/// A representation of a type, as it is known to the JIT compiler.
/// Types can be created through the Typeable trait or they can
/// be created dynamically by composing Field types.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Type<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_type>,
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct VectorType<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_vector_type>,
}

impl<'ctx> VectorType<'ctx> {
    unsafe fn from_ptr(ptr: *mut gccjit_sys::gcc_jit_vector_type) -> Option<VectorType<'ctx>> {
        Some(VectorType {
            marker: PhantomData,
            ptr: NonNull::new(ptr)?,
        })
    }

    pub fn get_element_type(&self) -> Option<Type<'ctx>> {
        let typ = with_lib_without_error_check(|lib| unsafe {
            from_ptr(lib.gcc_jit_vector_type_get_element_type(self.get_ptr()))
        })?;
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = typ.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
        Some(typ)
    }

    pub fn get_num_units(&self) -> usize {
        with_lib_without_error_check(|lib| unsafe {
            lib.gcc_jit_vector_type_get_num_units(self.get_ptr()) as usize
        })
    }

    fn get_ptr(&self) -> *mut gccjit_sys::gcc_jit_vector_type {
        self.ptr.as_ptr()
    }
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct FunctionPtrType<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: NonNull<gccjit_sys::gcc_jit_function_type>,
}

impl<'ctx> fmt::Debug for FunctionPtrType<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        write!(fmt, "{:?} (", self.get_return_type())?;
        for i in 0..self.get_param_count() {
            write!(fmt, "{:?}, ", self.get_param_type(i))?;
        }
        write!(fmt, ")")
    }
}

impl<'ctx> FunctionPtrType<'ctx> {
    unsafe fn from_ptr(
        ptr: *mut gccjit_sys::gcc_jit_function_type,
    ) -> Option<FunctionPtrType<'ctx>> {
        Some(FunctionPtrType {
            marker: PhantomData,
            ptr: NonNull::new(ptr)?,
        })
    }

    pub fn get_return_type(&self) -> Option<Type<'ctx>> {
        let typ = with_lib_without_error_check(|lib| unsafe {
            from_ptr(lib.gcc_jit_function_type_get_return_type(self.get_ptr()))
        })?;
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = typ.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
        Some(typ)
    }

    pub fn get_param_count(&self) -> usize {
        with_lib_without_error_check(|lib| unsafe {
            lib.gcc_jit_function_type_get_param_count(self.get_ptr()) as usize
        })
    }

    pub fn get_param_type(&self, index: usize) -> Option<Type<'ctx>> {
        let typ = with_lib_without_error_check(|lib| unsafe {
            from_ptr(lib.gcc_jit_function_type_get_param_type(self.get_ptr(), index as _))
        })?;
        #[cfg(debug_assertions)]
        if let Ok(Some(error)) = typ.to_object().get_context().get_last_error() {
            panic!("{}", error);
        }
        Some(typ)
    }

    fn get_ptr(&self) -> *mut gccjit_sys::gcc_jit_function_type {
        self.ptr.as_ptr()
    }
}

impl<'ctx> ToObject<'ctx> for Type<'ctx> {
    fn to_object(&self) -> Object<'ctx> {
        with_lib_without_error_check(|lib| unsafe {
            let ptr = lib.gcc_jit_type_as_object(get_ptr(self));
            object::from_ptr(ptr).expect("NULL type")
        })
    }
}

impl<'ctx> crate::ContextGetter<'ctx> for Type<'ctx> {
    fn context(&self) -> crate::ContextRef<'ctx> {
        self.to_object().context()
    }
}

impl<'ctx> fmt::Debug for Type<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        let obj = self.to_object();
        obj.fmt(fmt)
    }
}

impl<'ctx> Type<'ctx> {
    /// Given a type T, creates a type to *T, a pointer to T.
    pub fn make_pointer(self) -> Option<Type<'ctx>> {
        with_lib(&self, |lib| unsafe {
            from_ptr(lib.gcc_jit_type_get_pointer(get_ptr(&self)))
        })
    }

    #[cfg(feature = "master")]
    pub fn set_addressable(&self) {
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_type_set_addressable(get_ptr(self));
        })
    }

    /// Given a type T, creates a type of const T.
    pub fn make_const(self) -> Option<Type<'ctx>> {
        with_lib(&self, |lib| unsafe {
            from_ptr(lib.gcc_jit_type_get_const(get_ptr(&self)))
        })
    }

    /// Given a type T, creates a new type of volatile T, which
    /// has the semantics of C's volatile.
    pub fn make_volatile(self) -> Option<Type<'ctx>> {
        with_lib(&self, |lib| unsafe {
            from_ptr(lib.gcc_jit_type_get_volatile(get_ptr(&self)))
        })
    }

    /// Given a type T, creates a new type of restrict T, which
    /// has the semantics of C's restrict.
    #[cfg(feature = "master")]
    pub fn make_restrict(self) -> Option<Type<'ctx>> {
        with_lib(&self, |lib| unsafe {
            from_ptr(lib.gcc_jit_type_get_restrict(get_ptr(&self)))
        })
    }

    pub fn get_aligned(self, alignment_in_bytes: u64) -> Option<Type<'ctx>> {
        with_lib(&self, |lib| unsafe {
            from_ptr(lib.gcc_jit_type_get_aligned(get_ptr(&self), alignment_in_bytes as _))
        })
    }

    pub fn dyncast_array(self) -> Option<Type<'ctx>> {
        with_lib(&self, |lib| unsafe {
            let array_type = lib.gcc_jit_type_dyncast_array(get_ptr(&self));
            from_ptr(array_type)
        })
    }

    pub fn is_bool(self) -> bool {
        with_lib(&self, |lib| unsafe {
            lib.gcc_jit_type_is_bool(get_ptr(&self)) != 0
        })
    }

    pub fn is_integral(self) -> bool {
        with_lib(&self, |lib| unsafe {
            lib.gcc_jit_type_is_integral(get_ptr(&self)) != 0
        })
    }

    #[cfg(feature = "master")]
    pub fn is_floating_point(self) -> bool {
        with_lib(&self, |lib| unsafe {
            lib.gcc_jit_type_is_floating_point(get_ptr(&self)) != 0
        })
    }

    pub fn dyncast_vector(self) -> Option<VectorType<'ctx>> {
        with_lib(&self, |lib| unsafe {
            let vector_type = lib.gcc_jit_type_dyncast_vector(get_ptr(&self));
            VectorType::from_ptr(vector_type)
        })
    }

    pub fn is_struct(self) -> Option<Struct<'ctx>> {
        with_lib(&self, |lib| unsafe {
            let struct_type = lib.gcc_jit_type_is_struct(get_ptr(&self));
            structs::from_ptr(struct_type)
        })
    }

    pub fn dyncast_function_ptr_type(self) -> Option<FunctionPtrType<'ctx>> {
        with_lib(&self, |lib| unsafe {
            let function_ptr_type = lib.gcc_jit_type_dyncast_function_ptr_type(get_ptr(&self));
            FunctionPtrType::from_ptr(function_ptr_type)
        })
    }

    pub fn get_size(&self) -> u32 {
        with_lib(self, |lib| unsafe {
            let size = lib.gcc_jit_type_get_size(get_ptr(self));
            assert_ne!(size, -1, "called get_size of unsupported type: {self:?}");
            size as u32
        })
    }

    pub fn unqualified(&self) -> Option<Type<'ctx>> {
        with_lib(self, |lib| unsafe {
            from_ptr(lib.gcc_jit_type_unqualified(get_ptr(self)))
        })
    }

    pub fn get_pointee(&self) -> Option<Type<'ctx>> {
        with_lib(self, |lib| unsafe {
            let value = lib.gcc_jit_type_is_pointer(get_ptr(self));
            from_ptr(value)
        })
    }

    pub fn is_compatible_with(&self, typ: Type<'ctx>) -> bool {
        with_lib(self, |lib| unsafe {
            lib.gcc_jit_compatible_types(get_ptr(self), typ.ptr.as_ptr())
        })
    }

    #[cfg(feature = "master")]
    pub fn add_attribute(&self, attribute: TypeAttribute) {
        let value = attribute.get_value();
        with_lib(self, |lib| match value {
            AttributeValue::Int(value) => unsafe {
                lib.gcc_jit_type_add_integer_attribute(get_ptr(self), attribute.as_sys(), value);
            },
            AttributeValue::None => unsafe {
                lib.gcc_jit_type_add_attribute(get_ptr(self), attribute.as_sys());
            },
            AttributeValue::IntArray(_) => unimplemented!(),
            AttributeValue::String(_) => unimplemented!(),
        });
    }
}

#[cfg(feature = "master")]
#[derive(Clone, Debug)]
pub enum TypeAttribute {
    Aligned(u32),
    MayAlias,
    Packed,
}

#[cfg(feature = "master")]
impl TypeAttribute {
    fn get_value(&self) -> AttributeValue<'_> {
        match *self {
            Self::Aligned(value) => AttributeValue::Int(value as _),
            Self::MayAlias | Self::Packed => AttributeValue::None,
        }
    }

    fn as_sys(&self) -> gccjit_sys::gcc_jit_type_attribute {
        match *self {
            Self::Aligned(_) => gccjit_sys::gcc_jit_type_attribute::GCC_JIT_TYPE_ATTRIBUTE_ALIGNED,
            Self::MayAlias => gccjit_sys::gcc_jit_type_attribute::GCC_JIT_TYPE_ATTRIBUTE_MAY_ALIAS,
            Self::Packed => gccjit_sys::gcc_jit_type_attribute::GCC_JIT_TYPE_ATTRIBUTE_PACKED,
        }
    }
}

/// Typeable is a trait for types that have a corresponding type within
/// gccjit. This library implements this type for a variety of primitive types,
/// but it's also possible to implement this trait for more complex types
/// that will use the API on Context to construct analagous struct/union types.
pub trait Typeable {
    fn get_type<'a, 'ctx>(ctx: &'a Context<'ctx>) -> Option<Type<'a>>;
}

macro_rules! typeable_def {
    ($ty:ty, $expr:expr) => {
        impl Typeable for $ty {
            fn get_type<'a, 'ctx>(ctx: &'a Context<'ctx>) -> Option<Type<'a>> {
                with_lib(ctx, |lib| unsafe {
                    let ctx_ptr = context::get_ptr(ctx);
                    let ptr = lib.gcc_jit_context_get_type(ctx_ptr, $expr);
                    from_ptr(ptr)
                })
            }
        }
    };
}

typeable_def!((), GCC_JIT_TYPE_VOID);
typeable_def!(bool, GCC_JIT_TYPE_BOOL);
typeable_def!(char, GCC_JIT_TYPE_CHAR);
typeable_def!(f32, GCC_JIT_TYPE_FLOAT);
typeable_def!(f64, GCC_JIT_TYPE_DOUBLE);
typeable_def!(usize, GCC_JIT_TYPE_SIZE_T);

macro_rules! typeable_int_def {
    ($ty:ty, $num_bytes:expr, $signed:expr) => {
        impl Typeable for $ty {
            fn get_type<'a, 'ctx>(ctx: &'a Context<'ctx>) -> Option<Type<'a>> {
                with_lib(ctx, |lib| unsafe {
                    let ctx_ptr = context::get_ptr(ctx);
                    let ptr = lib.gcc_jit_context_get_int_type(ctx_ptr, $num_bytes, $signed as i32);
                    from_ptr(ptr)
                })
            }
        }
    };
}

typeable_int_def!(i8, 1, true);
typeable_int_def!(u8, 1, false);
typeable_int_def!(i16, 2, true);
typeable_int_def!(u16, 2, false);
typeable_int_def!(i32, 4, true);
typeable_int_def!(u32, 4, false);
typeable_int_def!(i64, 8, true);
typeable_int_def!(u64, 8, false);
//typeable_int_def!(i128, 16, true); // FIXME: unsupported by libgccjit for now.
//typeable_int_def!(u128, 16, false); // FIXME: unsupported by libgccjit for now.

/// Specific implementations of Typeable for *mut T and *const T that
/// represent void* and const void*, respectively. These impls should
/// only be used to expose opaque pointers to gccjit, not to create
/// pointers that are not opaque to gcc. For that, the make_pointer
/// function should be used.
impl<T> Typeable for *mut T {
    fn get_type<'a, 'ctx>(ctx: &'a Context<'ctx>) -> Option<Type<'a>> {
        with_lib(ctx, |lib| unsafe {
            let ctx_ptr = context::get_ptr(ctx);
            let ptr = lib.gcc_jit_context_get_type(ctx_ptr, GCC_JIT_TYPE_VOID_PTR);
            from_ptr(ptr)
        })
    }
}

impl<T> Typeable for *const T {
    fn get_type<'a, 'ctx>(ctx: &'a Context<'ctx>) -> Option<Type<'a>> {
        with_lib(ctx, |lib| unsafe {
            let ctx_ptr = context::get_ptr(ctx);
            let ptr = lib.gcc_jit_context_get_type(ctx_ptr, GCC_JIT_TYPE_VOID_PTR);
            from_ptr(ptr)?.make_const()
        })
    }
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_type) -> Option<Type<'ctx>> {
    Some(Type {
        marker: PhantomData,
        ptr: NonNull::new(ptr)?,
    })
}

pub unsafe fn get_ptr<'ctx>(ty: &Type<'ctx>) -> *mut gccjit_sys::gcc_jit_type {
    ty.ptr.as_ptr()
}

//! # gccjit.rs - Idiomatic Rust bindings to gccjit
//!
//! This library aims to provide idiomatic Rust bindings to gccjit,
//! the embeddable shared library that provides JIT compilation utilizing
//! GCC's backend. See https://gcc.gnu.org/wiki/JIT for more information
//! and for documentation of gccjit itself.
//!
//! Each one of the types provided in this crate corresponds to a pointer
//! type provided by the libgccjit C API. Type conversions are handled by
//! the ToRValue and ToLValue types, which represent values that can be
//! rvalues and values that can be lvalues, respectively.
//!
//! In addition, these types are all statically verified by the Rust compiler to
//! never outlive the Context object from which they came, a requirement
//! to using libgccjit correctly.

#![allow(clippy::needless_lifetimes)]

extern crate gccjit_sys;

mod asm;
mod types;
mod context;
mod object;
mod location;
mod field;
mod structs;
mod lvalue;
mod rvalue;
mod parameter;
mod function;
mod block;
#[cfg(feature="master")]
mod target_info;

#[cfg(feature="dlopen")]
use std::ffi::CStr;
#[cfg(feature="dlopen")]
use std::sync::OnceLock;

pub use context::Context;
pub use context::CType;
pub use context::GlobalKind;
pub use context::OptimizationLevel;
pub use context::CompileResult;
pub use context::OutputKind;
pub use location::Location;
pub use object::Object;
pub use object::ToObject;
pub use types::FunctionPtrType;
pub use types::Type;
pub use types::Typeable;
pub use field::Field;
pub use structs::Struct;
#[cfg(feature="master")]
pub use lvalue::{VarAttribute, Visibility};
pub use lvalue::{LValue, TlsModel, ToLValue};
pub use rvalue::{RValue, ToRValue};
pub use parameter::Parameter;
#[cfg(feature="master")]
pub use function::FnAttribute;
pub use function::{Function, FunctionType};
pub use block::{Block, BinaryOp, UnaryOp, ComparisonOp};
#[cfg(feature="master")]
pub use target_info::TargetInfo;

use gccjit_sys::Libgccjit;

#[cfg(feature="master")]
pub fn set_global_personality_function_name(name: &'static [u8]) {
    debug_assert!(name.ends_with(b"\0"), "Expecting a NUL-terminated C string");
    with_lib(|lib| {
        unsafe {
            lib.gcc_jit_set_global_personality_function_name(name.as_ptr() as *const _);
        }
    })
}

#[derive(Debug)]
pub struct Version {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
}

impl Version {
    pub fn get() -> Self {
        with_lib(|lib| {
            unsafe {
                Self {
                    major: lib.gcc_jit_version_major(),
                    minor: lib.gcc_jit_version_minor(),
                    patch: lib.gcc_jit_version_patchlevel(),
                }
            }
        })
    }
}

#[cfg(feature="master")]
pub fn is_lto_supported() -> bool {
    with_lib(|lib| {
        unsafe {
            lib.gcc_jit_is_lto_supported()
        }
    })
}

#[cfg(not(feature="dlopen"))]
fn with_lib<T, F: Fn(&Libgccjit) -> T>(callback: F) -> T {
    callback(&LIB)
}

#[cfg(feature="dlopen")]
fn with_lib<T, F: Fn(&Libgccjit) -> T>(callback: F) -> T {
    let lib = LIB.get().and_then(|lib| lib.as_ref());
    match lib {
        Some(lib) => callback(lib),
        None => panic!("libgccjit needs to be loaded by calling load() before attempting to do any operation"),
    }
}

/// Returns true if the library was loaded correctly, false otherwise.
#[cfg(feature="dlopen")]
pub fn load(path: &CStr) -> bool {
    let lib =
        LIB.get_or_init(|| {
            unsafe { Libgccjit::open(path) }
        });
    lib.is_some()
}

#[cfg(feature="dlopen")]
pub fn is_loaded() -> bool {
    LIB.get().is_some()
}

#[cfg(feature="dlopen")]
pub static LIB: OnceLock<Option<Libgccjit>> = OnceLock::new();

// Without the dlopen feature, we avoid using OnceLock as to not have any performance impact.
#[cfg(not(feature="dlopen"))]
static LIB: Libgccjit = Libgccjit::new();

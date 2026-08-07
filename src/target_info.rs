use context::CType;
use std::{
    ffi::{CStr, CString},
    fmt,
};

use crate::with_lib_without_error_check;

pub struct TargetInfo {
    ptr: *mut gccjit_sys::gcc_jit_target_info,
}

unsafe impl Send for TargetInfo {}
unsafe impl Sync for TargetInfo {}

impl fmt::Debug for TargetInfo {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        "TargetInfo".fmt(fmt)
    }
}

impl TargetInfo {
    pub fn cpu_supports(&self, feature: &str) -> bool {
        let feature = match CString::new(feature) {
            Ok(feature) => feature,
            Err(_) => return false,
        };
        with_lib_without_error_check(|lib| unsafe {
            lib.gcc_jit_target_info_cpu_supports(get_ptr(self), feature.as_ptr()) != 0
        })
    }

    pub fn arch(&self) -> Option<&'static CStr> {
        with_lib_without_error_check(|lib| unsafe {
            let arch = lib.gcc_jit_target_info_arch(get_ptr(self));
            if arch.is_null() {
                return None;
            }
            Some(CStr::from_ptr(arch))
        })
    }

    pub fn supports_target_dependent_type(&self, c_type: CType) -> bool {
        with_lib_without_error_check(|lib| unsafe {
            lib.gcc_jit_target_info_supports_target_dependent_type(get_ptr(self), c_type.to_sys())
                != 0
        })
    }
}

impl Drop for TargetInfo {
    fn drop(&mut self) {
        with_lib_without_error_check(|lib| unsafe {
            lib.gcc_jit_target_info_release(get_ptr(self));
        })
    }
}

pub unsafe fn from_ptr(ptr: *mut gccjit_sys::gcc_jit_target_info) -> TargetInfo {
    TargetInfo { ptr }
}

pub unsafe fn get_ptr(target: &TargetInfo) -> *mut gccjit_sys::gcc_jit_target_info {
    target.ptr
}

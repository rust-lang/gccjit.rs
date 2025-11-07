// TODO: add some safety to avoid being able to use a dangling symbol.

pub use self::platform::Library;

#[cfg(unix)]
mod platform {
    use std::ffi::{CStr, c_int, c_void};

    #[link(name="dl")]
    extern "C" {
        fn dlopen(filename: *const i8, flag: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
    }

    pub struct Library(*mut c_void);

    impl Library {
        pub unsafe fn open(path: &CStr) -> Option<Self> {
            const RTLD_NOW: c_int = 2;
            let handle = dlopen(path.as_ptr(), RTLD_NOW);
            if handle.is_null() {
                None
            }
            else {
                Some(Self(handle))
            }
        }

        pub unsafe fn get(&self, sym: &CStr) -> Option<*mut ()> {
            let ptr = dlsym(self.0, sym.as_ptr());
            if ptr.is_null() {
                None
            }
            else {
                Some(ptr.cast())
            }
        }
    }

    impl Drop for Library {
        fn drop(&mut self) {
            unsafe { dlclose(self.0); }
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::{CStr, OsString, c_void};
    use std::os::windows::ffi::OsStrExt;

    #[link(name="kernel32")]
    extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
        fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
        fn FreeLibrary(hLibModule: *mut c_void);
    }

    pub struct Library(*mut c_void);

    impl Library {
        pub unsafe fn open(path: &CStr) -> Option<Self> {
            let path = path.to_str().unwrap();
            let path = OsString::from(path);
            let w: Vec<u16> = path.encode_wide().collect();
            let handle = LoadLibraryW(w.as_ptr());
            if handle.is_null() {
                None
            }
            else {
                Some(Self(handle))
            }
        }

        pub unsafe fn get(&self, sym: &CStr) -> Option<*mut ()> {
            let ptr = GetProcAddress(self.0, sym.as_ptr() as *const _);
            if ptr.is_null() {
                None
            }
            else {
                Some(ptr.cast())
            }
        }
    }

    impl Drop for Library {
        fn drop(&mut self) {
            unsafe { FreeLibrary(self.0); }
        }
    }
}

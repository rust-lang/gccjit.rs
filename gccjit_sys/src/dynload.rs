// TODO: add some safety to avoid being able to use a dangling symbol.

pub use self::platform::Library;

#[cfg(unix)]
mod platform {
    use std::ffi::{CStr, c_char, c_int, c_void};

    #[link(name="dl")]
    extern "C" {
        fn dlopen(filename: *const i8, flag: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
        fn dlerror() -> *const c_char;
    }

    pub struct Library(*mut c_void);

    unsafe impl Send for Library {}
    unsafe impl Sync for Library {}

    impl Library {
        pub unsafe fn open(path: &CStr) -> Result<Self, String> {
            const RTLD_NOW: c_int = 2;
            let handle = dlopen(path.as_ptr(), RTLD_NOW);
            if handle.is_null() {
                Self::error()
            }
            else {
                Ok(Self(handle))
            }
        }

        pub unsafe fn get(&self, sym: &CStr) -> Result<*mut (), String> {
            let ptr = dlsym(self.0, sym.as_ptr());
            if ptr.is_null() {
                Self::error()
            }
            else {
                Ok(ptr.cast())
            }
        }

        unsafe fn error<T>() -> Result<T, String> {
            let cstr = dlerror();
            let cstr = CStr::from_ptr(cstr);
            let string = cstr.to_str()
                .map_err(|error| error.to_string())?
                .to_string();
            Err(string)
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

    unsafe impl Send for Library {}
    unsafe impl Sync for Library {}

    impl Library {
        pub unsafe fn open(path: &CStr) -> Result<Self, String> {
            let path = path.to_str().unwrap();
            let path = OsString::from(path);
            let w: Vec<u16> = path.encode_wide().collect();
            let handle = LoadLibraryW(w.as_ptr());
            if handle.is_null() {
                Err("cannot load library".to_string())
            }
            else {
                Ok(Self(handle))
            }
        }

        pub unsafe fn get(&self, sym: &CStr) -> Result<*mut (), String> {
            let ptr = GetProcAddress(self.0, sym.as_ptr() as *const _);
            if ptr.is_null() {
                Some("cannot load symbol".to_string())
            }
            else {
                Ok(ptr.cast())
            }
        }
    }

    impl Drop for Library {
        fn drop(&mut self) {
            unsafe { FreeLibrary(self.0); }
        }
    }
}

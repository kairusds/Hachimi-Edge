use std::{ffi::CString, os::raw::c_void};

/// Resolve a symbol from an already-loaded dylib handle.
/// Mirrors Android symbols_impl — iOS is POSIX so libc::dlsym works identically.
pub unsafe fn dlsym(handle: *mut c_void, name: &str) -> usize {
    debug_assert!(!handle.is_null());
    let name_cstr = CString::new(name).unwrap();
    libc::dlsym(handle, name_cstr.as_ptr()) as usize
}

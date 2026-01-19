use std::{ffi::CStr, os::raw::c_void};
use jni::{sys::jint, JavaVM};
use once_cell::sync::OnceCell;

use crate::core::Hachimi;

use super::{hook, plugin_loader};

#[allow(non_camel_case_types)]
type JniOnLoadFn = extern "C" fn(vm: JavaVM, reserved: *mut c_void) -> jint;

const LIBRARY_NAME: &CStr = c"libmain_orig.so";
const JNI_ONLOAD_NAME: &CStr = c"JNI_OnLoad";

static JAVA_VM: OnceCell<JavaVM> = OnceCell::new();

pub(crate) fn java_vm() -> Option<&'static JavaVM> {
    JAVA_VM.get()
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: JavaVM, reserved: *mut c_void) -> jint {
    let orig_fn: JniOnLoadFn;
    unsafe {
        let handle = libc::dlopen(LIBRARY_NAME.as_ptr(), libc::RTLD_LAZY);
        orig_fn = std::mem::transmute(libc::dlsym(handle, JNI_ONLOAD_NAME.as_ptr()));
    }

    if !Hachimi::init() {
        return orig_fn(vm, reserved);
    }
    let vm_ptr = vm.get_java_vm_pointer();
    let vm_for_env = unsafe { JavaVM::from_raw(vm_ptr).unwrap() };
    let _ = JAVA_VM.set(vm);
    let hachimi = Hachimi::instance();
    *hachimi.plugins.lock().unwrap() = plugin_loader::load_libraries();
    let env = vm_for_env.get_env().unwrap();
    hook::init(env.get_raw());

    info!("JNI_OnLoad");
    let vm_for_orig = unsafe { JavaVM::from_raw(vm_ptr).unwrap() };
    orig_fn(vm_for_orig, reserved)
}

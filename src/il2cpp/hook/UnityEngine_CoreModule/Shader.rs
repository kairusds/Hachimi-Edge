use std::ptr::null_mut;

use crate::il2cpp::{ext::StringExt, symbols::get_method_addr, types::*};

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_method_wrapper_fn!(Find, FIND_ADDR, *mut Il2CppObject, name: *mut Il2CppString);

pub fn find(name: &str) -> *mut Il2CppObject {
    Find(name.to_il2cpp_string())
}

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Shader);

    unsafe {
        CLASS = Shader;
        FIND_ADDR = get_method_addr(Shader, c"Find", 1);
    }
}

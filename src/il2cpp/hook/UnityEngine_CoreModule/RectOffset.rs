use crate::il2cpp::{api::il2cpp_object_new, symbols::get_method_addr, types::*};

static mut CLASS: *mut Il2CppClass = std::ptr::null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut CTOR_ADDR: usize = 0;
impl_addr_wrapper_fn!(_ctor, CTOR_ADDR, (),
    this: *mut Il2CppObject, left: i32, right: i32, top: i32, bottom: i32
);

pub fn new(left: i32, right: i32, top: i32, bottom: i32) -> *mut Il2CppObject {
    let this = il2cpp_object_new(class());
    _ctor(this, left, right, top, bottom);
    this
}

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, RectOffset);

    unsafe {
        CLASS = RectOffset;
        CTOR_ADDR = get_method_addr(RectOffset, c".ctor", 4);
    }
}

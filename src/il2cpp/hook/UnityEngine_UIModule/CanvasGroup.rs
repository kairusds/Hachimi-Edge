use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GET_ALPHA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_alpha, GET_ALPHA_ADDR, f32, this: *mut Il2CppObject);

static mut SET_ALPHA_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_alpha, SET_ALPHA_ADDR, (), this: *mut Il2CppObject, value: f32);

pub fn init(UnityEngine_UIModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UIModule, UnityEngine, CanvasGroup);

    unsafe {
        CLASS = CanvasGroup;
        GET_ALPHA_ADDR = get_method_addr(CanvasGroup, c"get_alpha", 0);
        SET_ALPHA_ADDR = get_method_addr(CanvasGroup, c"set_alpha", 1);
    }
}

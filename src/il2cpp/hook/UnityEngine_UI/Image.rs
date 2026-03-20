use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut SET_TYPE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_type, SET_TYPE_ADDR, (), this: *mut Il2CppObject, value: i32);

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.UI", Image);
    
    unsafe {
        SET_TYPE_ADDR = get_method_addr(Image, c"set_type", 1);
    }
}
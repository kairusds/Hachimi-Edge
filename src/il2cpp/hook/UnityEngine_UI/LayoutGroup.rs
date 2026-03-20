use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut SET_PADDING_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_padding, SET_PADDING_ADDR, (), this: *mut Il2CppObject, value: *mut Il2CppObject);

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.UI", LayoutGroup);
    
    unsafe {
        SET_PADDING_ADDR = get_method_addr(LayoutGroup, c"set_padding", 1);
    }
}

use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut SET_CHILDFORCEEXPANDHEIGHT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_childForceExpandHeight, SET_CHILDFORCEEXPANDHEIGHT_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut SET_CHILDCONTROLHEIGHT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_childControlHeight, SET_CHILDCONTROLHEIGHT_ADDR, (), this: *mut Il2CppObject, value: bool);

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.UI", HorizontalOrVerticalLayoutGroup);
    
    unsafe {
        SET_CHILDFORCEEXPANDHEIGHT_ADDR = get_method_addr(HorizontalOrVerticalLayoutGroup, c"set_childForceExpandHeight", 1);
        SET_CHILDCONTROLHEIGHT_ADDR = get_method_addr(HorizontalOrVerticalLayoutGroup, c"set_childControlHeight", 1);
    }
}

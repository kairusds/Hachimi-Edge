use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut FORCEREBUILDLAYOUTIMMEDIATE_ADDR: usize = 0;
impl_addr_wrapper_fn!(ForceRebuildLayoutImmediate, FORCEREBUILDLAYOUTIMMEDIATE_ADDR, (), layoutRoot: *mut Il2CppObject);

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.UI", LayoutRebuilder);
    
    unsafe {
        FORCEREBUILDLAYOUTIMMEDIATE_ADDR = get_method_addr(LayoutRebuilder, c"ForceRebuildLayoutImmediate", 1);
    }
}

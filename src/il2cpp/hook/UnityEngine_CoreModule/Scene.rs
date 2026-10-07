use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut GET_NAMEINTERNAL_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetNameInternal, GET_NAMEINTERNAL_ADDR, *mut Il2CppString, handle: i32);

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, "UnityEngine.SceneManagement", Scene);

    unsafe {
        GET_NAMEINTERNAL_ADDR = get_method_addr(Scene, c"GetNameInternal", 1);
    }
}
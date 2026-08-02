use crate::il2cpp::{symbols::get_method_addr, types::*};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Scene_t {
    pub handle: i32,
}

static mut GET_ACTIVESCENE_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetActiveScene, GET_ACTIVESCENE_ADDR, Scene_t, );

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, "UnityEngine.SceneManagement", SceneManager);

    unsafe {
        GET_ACTIVESCENE_ADDR = get_method_addr(SceneManager, c"GetActiveScene", 0);
    }
}

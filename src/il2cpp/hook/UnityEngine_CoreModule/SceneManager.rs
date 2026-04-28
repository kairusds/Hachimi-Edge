use crate::il2cpp::{symbols::{get_class, get_method_addr}, types::*};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Scene_t {
    pub handle: i32,
}

static mut GET_ACTIVESCENE_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetActiveScene, GET_ACTIVESCENE_ADDR, Scene_t, );

pub fn init(image: *const Il2CppImage) {
    if let Ok(klass) = get_class(image, c"UnityEngine.SceneManagement", c"SceneManager") {
        unsafe {
            GET_ACTIVESCENE_ADDR = get_method_addr(klass, c"GetActiveScene", 0);
        }
    }
}
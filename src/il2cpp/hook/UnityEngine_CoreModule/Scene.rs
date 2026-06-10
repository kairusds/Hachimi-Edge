use crate::il2cpp::{symbols::{get_class, get_method_addr}, types::*};

static mut GET_NAMEINTERNAL_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetNameInternal, GET_NAMEINTERNAL_ADDR, *mut Il2CppString, handle: i32);

pub fn init(image: *const Il2CppImage) {
    if let Ok(klass) = get_class(image, c"UnityEngine.SceneManagement", c"Scene") {
        unsafe {
            GET_NAMEINTERNAL_ADDR = get_method_addr(klass, c"GetNameInternal", 1);
        }
    }
}
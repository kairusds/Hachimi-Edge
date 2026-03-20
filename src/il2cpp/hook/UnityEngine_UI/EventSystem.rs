use crate::il2cpp::{symbols::{get_method_addr}, types::*};

static mut GET_CURRENT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_current, GET_CURRENT_ADDR, *mut Il2CppObject,);

static mut GET_CURRENTSELECTEDGAMEOBJECT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_currentSelectedGameObject, GET_CURRENTSELECTEDGAMEOBJECT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.EventSystems", EventSystem);

    unsafe {
        GET_CURRENT_ADDR = get_method_addr(EventSystem, c"get_current", 0);
        GET_CURRENTSELECTEDGAMEOBJECT_ADDR = get_method_addr(EventSystem, c"get_currentSelectedGameObject", 0);
    }
}
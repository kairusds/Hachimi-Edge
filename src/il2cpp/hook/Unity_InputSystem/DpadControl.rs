use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut GET_UP_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_up, GET_UP_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_DOWN_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_down, GET_DOWN_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_LEFT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_left, GET_LEFT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_RIGHT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_right, GET_RIGHT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(Unity_InputSystem: *const Il2CppImage) {
    get_class_or_return!(Unity_InputSystem, "UnityEngine.InputSystem.Controls", DpadControl);

    unsafe {
        GET_UP_ADDR = get_method_addr(DpadControl, c"get_up", 0);
        GET_DOWN_ADDR = get_method_addr(DpadControl, c"get_down", 0);
        GET_LEFT_ADDR = get_method_addr(DpadControl, c"get_left", 0);
        GET_RIGHT_ADDR = get_method_addr(DpadControl, c"get_right", 0);
    }
}

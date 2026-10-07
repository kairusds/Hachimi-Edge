use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut GET_IS_PRESSED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_isPressed, GET_IS_PRESSED_ADDR, bool, this: *mut Il2CppObject);

pub fn init(Unity_InputSystem: *const Il2CppImage) {
    get_class_or_return!(Unity_InputSystem, "UnityEngine.InputSystem.Controls", ButtonControl);

    unsafe {
        GET_IS_PRESSED_ADDR = get_method_addr(ButtonControl, c"get_isPressed", 0);
    }
}

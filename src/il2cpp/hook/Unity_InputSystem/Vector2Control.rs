use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut GET_X_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_x, GET_X_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_Y_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_y, GET_Y_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(Unity_InputSystem: *const Il2CppImage) {
    get_class_or_return!(Unity_InputSystem, "UnityEngine.InputSystem.Controls", Vector2Control);

    unsafe {
        GET_X_ADDR = get_method_addr(Vector2Control, c"get_x", 0);
        GET_Y_ADDR = get_method_addr(Vector2Control, c"get_y", 0);
    }
}

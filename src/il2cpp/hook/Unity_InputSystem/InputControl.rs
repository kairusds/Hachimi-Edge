use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut GET_CURRENT_STATE_PTR_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_currentStatePtr, GET_CURRENT_STATE_PTR_ADDR, *mut std::ffi::c_void, this: *mut Il2CppObject);

pub fn init(Unity_InputSystem: *const Il2CppImage) {
    get_class_or_return!(Unity_InputSystem, "UnityEngine.InputSystem", InputControl);

    unsafe {
        GET_CURRENT_STATE_PTR_ADDR = get_method_addr(InputControl, c"get_currentStatePtr", 0);
    }
}

use std::ffi::c_void;
use crate::il2cpp::{symbols::get_method_addr, types::*};
use super::InputControl;

static mut READ_AXIS_UNPROCESSED_ADDR: usize = 0;
impl_addr_wrapper_fn!(ReadUnprocessedValueFromState, READ_AXIS_UNPROCESSED_ADDR, f32, this: *mut Il2CppObject, state: *mut c_void);

pub fn read_unprocessed_value(control: *mut Il2CppObject) -> f32 {
    if control.is_null() {
        return 0.0;
    }
    let state = InputControl::get_currentStatePtr(control);
    if state.is_null() {
        return 0.0;
    }
    ReadUnprocessedValueFromState(control, state)
}

pub fn init(Unity_InputSystem: *const Il2CppImage) {
    get_class_or_return!(Unity_InputSystem, "UnityEngine.InputSystem.Controls", AxisControl);

    unsafe {
        READ_AXIS_UNPROCESSED_ADDR = get_method_addr(AxisControl, c"ReadUnprocessedValueFromState", 1);
    }
}

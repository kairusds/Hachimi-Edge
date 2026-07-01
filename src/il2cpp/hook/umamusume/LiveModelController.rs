use std::ptr::null_mut;

use crate::il2cpp::{
    symbols::{get_class, get_method_addr},
    types::*,
};

static mut GET_HEAD_TRANSFORM_ADDR: usize = 0;
static mut SET_MESH_ACTIVE_ADDR: usize = 0;

pub fn get_HeadTransform(this: *mut Il2CppObject) -> *mut Il2CppObject {
    if unsafe { GET_HEAD_TRANSFORM_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(GET_HEAD_TRANSFORM_ADDR) };
    func(this)
}

pub fn SetMeshActive(this: *mut Il2CppObject, is_active: bool) {
    if unsafe { SET_MESH_ACTIVE_ADDR } == 0 {
        return;
    }
    let func: extern "C" fn(*mut Il2CppObject, bool) =
        unsafe { std::mem::transmute(SET_MESH_ACTIVE_ADDR) };
    func(this, is_active);
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(live_model_controller) = get_class(umamusume, c"Gallop", c"LiveModelController") {
        unsafe {
            GET_HEAD_TRANSFORM_ADDR =
                get_method_addr(live_model_controller, c"get_HeadTransform", 0);
            SET_MESH_ACTIVE_ADDR = get_method_addr(live_model_controller, c"SetMeshActive", 1);
        }
    }
}

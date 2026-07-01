use std::ptr::null_mut;

use crate::il2cpp::{
    symbols::{get_class, get_method_addr},
    types::*,
};

static mut GET_OWNER_OBJECT_ADDR: usize = 0;
static mut SET_VISIBLE_ADDR: usize = 0;

pub fn get_OwnerObject(this: *mut Il2CppObject) -> *mut Il2CppObject {
    if unsafe { GET_OWNER_OBJECT_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(GET_OWNER_OBJECT_ADDR) };
    func(this)
}

pub fn SetVisible(this: *mut Il2CppObject, visible: bool, force: bool) {
    if unsafe { SET_VISIBLE_ADDR } == 0 {
        return;
    }
    let func: extern "C" fn(*mut Il2CppObject, bool, bool) =
        unsafe { std::mem::transmute(SET_VISIBLE_ADDR) };
    func(this, visible, force);
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(model_controller) = get_class(umamusume, c"Gallop", c"ModelController") {
        unsafe {
            GET_OWNER_OBJECT_ADDR = get_method_addr(model_controller, c"get_OwnerObject", 0);
            SET_VISIBLE_ADDR = get_method_addr(model_controller, c"SetVisible", 2);
        }
    }
}

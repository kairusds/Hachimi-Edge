use std::ptr::null_mut;

use crate::il2cpp::{
    symbols::{get_class, get_method_addr},
    types::*,
};

static mut GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR: usize = 0;

pub fn get_LiveModelControllerArray(this: *mut Il2CppObject) -> *mut Il2CppObject {
    if unsafe { GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR) };
    func(this)
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(character_object) = get_class(umamusume, c"Gallop.Live", c"CharacterObject") {
        unsafe {
            GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR =
                get_method_addr(character_object, c"get_LiveModelControllerArray", 0);
        }
    }
}

use crate::il2cpp::{
    symbols::get_method_addr,
    types::*,
};

static mut GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_LiveModelControllerArray, GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut SET_LIVE_CHARA_VISIBLE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_liveCharaVisible, SET_LIVE_CHARA_VISIBLE_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut APPLY_VISIBLE_ADDR: usize = 0;
impl_addr_wrapper_fn!(ApplyVisible, APPLY_VISIBLE_ADDR, (), this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live", CharacterObject);

    unsafe {
        GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR = get_method_addr(CharacterObject, c"get_LiveModelControllerArray", 0);
        SET_LIVE_CHARA_VISIBLE_ADDR = get_method_addr(CharacterObject, c"set_liveCharaVisible", 1);
        APPLY_VISIBLE_ADDR = get_method_addr(CharacterObject, c"ApplyVisible", 0);
    }
}

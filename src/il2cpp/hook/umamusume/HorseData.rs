use crate::il2cpp::{
    symbols::{get_class, get_method_addr},
    types::*,
};

static mut GET_GATE_NO_ADDR: usize = 0;

pub fn get_GateNo(this: *mut Il2CppObject) -> Option<i32> {
    if unsafe { GET_GATE_NO_ADDR } == 0 {
        return None;
    }
    let func: extern "C" fn(*mut Il2CppObject) -> i32 =
        unsafe { std::mem::transmute(GET_GATE_NO_ADDR) };
    Some(func(this))
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(horse_data) = get_class(umamusume, c"Gallop", c"HorseData") {
        unsafe {
            GET_GATE_NO_ADDR = get_method_addr(horse_data, c"get_GateNo", 0);
        }
    }
}

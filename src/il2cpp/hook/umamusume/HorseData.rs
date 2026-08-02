use crate::il2cpp::{
    symbols::get_method_addr,
    types::*,
};

static mut GET_GATE_NO_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_GateNo, GET_GATE_NO_ADDR, i32, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, HorseData);

    unsafe {
        GET_GATE_NO_ADDR = get_method_addr(HorseData, c"get_GateNo", 0);
    }
}

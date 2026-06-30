use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static mut GET_CHILDCURRENTCONTROLLER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_ChildCurrentController, GET_CHILDCURRENTCONTROLLER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, HubViewControllerBase);

    unsafe {
        GET_CHILDCURRENTCONTROLLER_ADDR = get_method_addr(HubViewControllerBase, c"get_ChildCurrentController", 0);
    }
}

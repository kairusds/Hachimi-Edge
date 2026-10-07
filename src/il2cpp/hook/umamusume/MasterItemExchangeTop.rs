use crate::{
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
};

// public bool get_IsInTermAnyAnnivShop() { }
static mut GET_ISINTERMANYANNIVSHOP_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_IsInTermAnyAnnivShop, GET_ISINTERMANYANNIVSHOP_ADDR, bool, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, MasterItemExchangeTop);

    unsafe {
        GET_ISINTERMANYANNIVSHOP_ADDR = get_method_addr(MasterItemExchangeTop, c"get_IsInTermAnyAnnivShop", 0);
    }
}

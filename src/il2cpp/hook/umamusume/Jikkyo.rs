use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_method_wrapper_fn!(SkipToStart, SKIP_TO_START_ADDR, (), this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, Jikkyo);

    unsafe {
        CLASS = Jikkyo;
        SKIP_TO_START_ADDR = get_method_addr(Jikkyo, c"SkipToStart", 0);
    }
}

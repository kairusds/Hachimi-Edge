use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GETTOPUI_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetTopUI, GETTOPUI_ADDR, *mut Il2CppObject, this: *mut Il2CppObject, index: i32);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, HomeViewController);

    unsafe {
        CLASS = HomeViewController;
        GETTOPUI_ADDR = get_method_addr(HomeViewController, c"GetTopUI", 1);
    }
}

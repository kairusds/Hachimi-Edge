use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_method_wrapper_fn!(ClearDisplay, CLEAR_DISPLAY_ADDR, (), this: *mut Il2CppObject);
def_method_wrapper_fn!(ClearVoice, CLEAR_VOICE_ADDR, (), this: *mut Il2CppObject);
def_method_wrapper_fn!(ClearReserve, CLEAR_RESERVE_ADDR, (), this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, JikkyoControllerBase);

    unsafe {
        CLASS = JikkyoControllerBase;
        CLEAR_DISPLAY_ADDR = get_method_addr(JikkyoControllerBase, c"ClearDisplay", 0);
        CLEAR_VOICE_ADDR = get_method_addr(JikkyoControllerBase, c"ClearVoice", 0);
        CLEAR_RESERVE_ADDR = get_method_addr(JikkyoControllerBase, c"ClearReserve", 0);
    }
}

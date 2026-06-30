use crate::il2cpp::{
    symbols::{get_method_addr, get_field_from_name},
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut PAUSELIVE_ADDR: usize = 0;
impl_addr_wrapper_fn!(PauseLive, PAUSELIVE_ADDR, (), this: *mut Il2CppObject);

static mut RESUMELIVE_ADDR: usize = 0;
impl_addr_wrapper_fn!(ResumeLive, RESUMELIVE_ADDR, (), this: *mut Il2CppObject);

static mut SKIPLIVE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SkipLive, SKIPLIVE_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GETVIEWBASE_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetViewBase, GETVIEWBASE_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

def_field_value_accessors!(get__state, set__state, _STATE_FIELD, i32);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, LiveViewController);

    unsafe {
        CLASS = LiveViewController;
        PAUSELIVE_ADDR = get_method_addr(LiveViewController, c"PauseLive", 0);
        RESUMELIVE_ADDR = get_method_addr(LiveViewController, c"ResumeLive", 0);
        SKIPLIVE_ADDR = get_method_addr(LiveViewController, c"SkipLive", 0);
        GETVIEWBASE_ADDR = get_method_addr(LiveViewController, c"GetViewBase", 0);
        _STATE_FIELD = get_field_from_name(LiveViewController, c"_state");
    }
}

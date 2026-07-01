use std::ptr::null_mut;

use crate::il2cpp::{
    symbols::{get_method_addr, SingletonLike},
    types::*,
};

static mut CLASS: *mut Il2CppClass = null_mut();
static mut REFRESH_ALL_ADDR: usize = 0;

pub fn refresh_all() {
    let class = unsafe { CLASS };
    if class.is_null() || unsafe { REFRESH_ALL_ADDR } == 0 {
        return;
    }

    let Some(singleton) = SingletonLike::new(class) else {
        return;
    };
    let instance = singleton.instance();
    if instance.is_null() {
        return;
    }

    let refresh_all: extern "C" fn(*mut Il2CppObject) = unsafe { std::mem::transmute(REFRESH_ALL_ADDR) };
    refresh_all(instance);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TapEffectController);

    unsafe {
        CLASS = TapEffectController;
        REFRESH_ALL_ADDR = get_method_addr(TapEffectController, c"RefreshAll", 0);
    }
}

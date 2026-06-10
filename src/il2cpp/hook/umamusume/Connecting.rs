use crate::{core::taskbar::{self, TBPF_INDETERMINATE, TBPF_NOPROGRESS}, il2cpp::{symbols::get_method_addr, types::*}};

type ShowFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Show(this: *mut Il2CppObject) {
    taskbar::update_connecting_state(TBPF_INDETERMINATE);
    get_orig_fn!(Show, ShowFn)(this);
}

type HideFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Hide(this: *mut Il2CppObject) {
    taskbar::update_connecting_state(TBPF_NOPROGRESS);
    get_orig_fn!(Hide, HideFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, Connecting);
    let Show_addr = get_method_addr(Connecting, c"Show", 0);
    let Hide_addr = get_method_addr(Connecting, c"Hide", 0);
    new_hook!(Show_addr, Show);
    new_hook!(Hide_addr, Hide);
}
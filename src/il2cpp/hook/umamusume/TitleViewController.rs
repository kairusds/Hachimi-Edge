use crate::{core::taskbar::{self, TBPF_NOPROGRESS}, il2cpp::{symbols::get_method_addr, types::*}};

use super::MainGameInitializer;

type UpdateViewFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn UpdateView(this: *mut Il2CppObject) {
    get_orig_fn!(UpdateView, UpdateViewFn)(this);
    if MainGameInitializer::GetBootProgress() != 0.0 {
        let progress = MainGameInitializer::GetBootProgress();
        if progress >= 0.0 {
            if progress >= 1.0 {
                taskbar::update_download_state(TBPF_NOPROGRESS);
            } else {
                taskbar::update_download_value((progress * 100.0) as u64, 100);
            }
        }
    }
}


pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TitleViewController);

    let UpdateView_addr = get_method_addr(TitleViewController, c"UpdateView", 0);
    new_hook!(UpdateView_addr, UpdateView);
}

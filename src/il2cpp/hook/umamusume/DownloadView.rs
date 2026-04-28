use crate::{core::taskbar::{self, TBPF_NOPROGRESS}, il2cpp::{symbols::get_method_addr, types::*}};

type UpdateViewFn = extern "C" fn(this: *mut Il2CppObject, download_size: f32, all_download_size: f32);
extern "C" fn UpdateView(this: *mut Il2CppObject, download_size: f32, all_download_size: f32) {
    let progress = download_size / all_download_size;
    if progress >= 1.0 {
        taskbar::update_download_state(TBPF_NOPROGRESS);
    } else {
        taskbar::update_download_value((progress * 10000.0) as u64, 10000);
    }
    get_orig_fn!(UpdateView, UpdateViewFn)(this, download_size, all_download_size);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DownloadView);
    let UpdateView_addr = get_method_addr(DownloadView, c"UpdateView", 2);
    new_hook!(UpdateView_addr, UpdateView);
}
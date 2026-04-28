use crate::{core::taskbar::{self, TBPF_NORMAL, TBPF_NOPROGRESS}, il2cpp::{symbols::get_method_addr, types::*}};

type ShowFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Show(this: *mut Il2CppObject) {
    taskbar::update_download_state(TBPF_NORMAL);
    get_orig_fn!(Show, ShowFn)(this);
}

type HideFn = extern "C" fn(this: *mut Il2CppObject, is_delay: bool);
extern "C" fn Hide(this: *mut Il2CppObject, is_delay: bool) {
    taskbar::update_download_state(TBPF_NOPROGRESS);
    get_orig_fn!(Hide, HideFn)(this, is_delay);
}

type SetProgressFn = extern "C" fn(this: *mut Il2CppObject, progress: f32);
extern "C" fn SetProgress(this: *mut Il2CppObject, progress: f32) {
    taskbar::update_download_value((progress * 10000.0) as u64, 10000);
    get_orig_fn!(SetProgress, SetProgressFn)(this, progress);
}

pub fn init(DownloadManager: *mut Il2CppClass) {
    find_nested_class_or_return!(DownloadManager, BackgroundDownloadProgressUI);

    let show_addr = get_method_addr(BackgroundDownloadProgressUI, c"Show", 0);
    let hide_addr = get_method_addr(BackgroundDownloadProgressUI, c"Hide", 1);
    let set_progress_addr = get_method_addr(BackgroundDownloadProgressUI, c"SetProgress", 1);

    new_hook!(show_addr, Show);
    new_hook!(hide_addr, Hide);
    new_hook!(set_progress_addr, SetProgress);
}
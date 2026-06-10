use crate::{core::taskbar::{self, TBPF_ERROR}, il2cpp::{symbols::get_method_addr, types::*}};

type ExecDownloadErrorProcessFn = extern "C" fn(error: *mut Il2CppObject, on_retry: *mut Il2CppObject, on_goto_title: *mut Il2CppObject);
extern "C" fn ExecDownloadErrorProcess(error: *mut Il2CppObject, on_retry: *mut Il2CppObject, on_goto_title: *mut Il2CppObject) {
    taskbar::update_download_state(TBPF_ERROR);
    get_orig_fn!(ExecDownloadErrorProcess, ExecDownloadErrorProcessFn)(error, on_retry, on_goto_title);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DownloadErrorProcessor);
    let ExecDownloadErrorProcess_addr = get_method_addr(DownloadErrorProcessor, c"ExecDownloadErrorProcess", 3);
    new_hook!(ExecDownloadErrorProcess_addr, ExecDownloadErrorProcess);
}
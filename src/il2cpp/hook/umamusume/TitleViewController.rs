use crate::{core::taskbar::{self, TBPF_NOPROGRESS}, il2cpp::{symbols::get_method_addr, types::*}};

static mut GET_BOOT_PROGRESS_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_boot_progress, GET_BOOT_PROGRESS_ADDR, f32, );

type UpdateViewFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn UpdateView(this: *mut Il2CppObject) {
    get_orig_fn!(UpdateView, UpdateViewFn)(this);
    unsafe {
        if GET_BOOT_PROGRESS_ADDR != 0 {
            let progress = get_boot_progress();
            if progress >= 0.0 {
                if progress >= 1.0 {
                    taskbar::update_download_state(TBPF_NOPROGRESS);
                } else {
                    taskbar::update_download_value((progress * 100.0) as u64, 100);
                }
            }
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TitleViewController);
    let UpdateView_addr = get_method_addr(TitleViewController, c"UpdateView", 0);
    new_hook!(UpdateView_addr, UpdateView);

    if let Ok(main_game_init) = crate::il2cpp::symbols::get_class(umamusume, c"Gallop", c"MainGameInitializer") {
        unsafe { GET_BOOT_PROGRESS_ADDR = get_method_addr(main_game_init, c"GetBootProgress", 0); }
    }
}
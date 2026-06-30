use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GETCURRENTBGMMUSICID_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetCurrentBgmMusicId, GETCURRENTBGMMUSICID_ADDR, i32, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, WorkJukeboxData);

    unsafe {
        CLASS = WorkJukeboxData;
        GETCURRENTBGMMUSICID_ADDR = get_method_addr(WorkJukeboxData, c"GetCurrentBgmMusicId", 0);
    }
}

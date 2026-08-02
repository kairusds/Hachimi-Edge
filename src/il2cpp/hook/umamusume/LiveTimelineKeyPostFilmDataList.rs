use crate::{
    il2cpp::{
        symbols::get_method_addr,
        types::*,
    },
};

static mut CLEAR_ADDR: usize = 0;
impl_addr_wrapper_fn!(Clear, CLEAR_ADDR, (), this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live.Cutt", LiveTimelineKeyPostFilmDataList);

    unsafe {
        CLEAR_ADDR = get_method_addr(LiveTimelineKeyPostFilmDataList, c"Clear", 0);
    }
}

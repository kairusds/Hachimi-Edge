use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

// public Void PlayCoroutinePlaySetList(Boolean stopPlay, Single startTime, Boolean setPlayInfo)
static mut PLAY_COROUTINE_PLAY_SET_LIST_ADDR: usize = 0;
impl_addr_wrapper_fn!(PlayCoroutinePlaySetList, PLAY_COROUTINE_PLAY_SET_LIST_ADDR, (), this: *mut Il2CppObject, stop_play: bool, start_time: f32, set_play_info: bool);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, JukeboxBgmSelector);

    unsafe {
        PLAY_COROUTINE_PLAY_SET_LIST_ADDR = get_method_addr(JukeboxBgmSelector, c"PlayCoroutinePlaySetList", 3);
    }
}

use crate::{
    core::live_utils::CriAtomExPlayback,
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;

pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GET_PLAYER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_player, GET_PLAYER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut SET_PLAYBACK_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_Playback, SET_PLAYBACK_ADDR, (), this: *mut Il2CppObject, value: CriAtomExPlayback);

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", AtomSourceEx);

    unsafe {
        CLASS = AtomSourceEx;
        GET_PLAYER_ADDR = get_method_addr(AtomSourceEx, c"get_player", 0);
        SET_PLAYBACK_ADDR = get_method_addr(AtomSourceEx, c"set_Playback", 1);
    }
}

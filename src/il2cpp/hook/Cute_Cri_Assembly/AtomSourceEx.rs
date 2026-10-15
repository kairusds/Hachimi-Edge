use crate::{
    il2cpp::{
        hook::CriMw_CriWare_Runtime::CriAtomExPlayback::CriAtomExPlayback_t,
        symbols::get_method_addr,
        types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;

pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_method_wrapper_fn!(get_player, GET_PLAYER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(set_Playback, SET_PLAYBACK_ADDR, (), this: *mut Il2CppObject, value: CriAtomExPlayback_t);
def_method_wrapper_fn!(get_IsInUse, GET_ISINUSE_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(Stop, STOP_ADDR, (), this: *mut Il2CppObject, fade_out_time: f32, fade_curve: i32);

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", AtomSourceEx);

    unsafe {
        CLASS = AtomSourceEx;
        GET_PLAYER_ADDR = get_method_addr(AtomSourceEx, c"get_player", 0);
        SET_PLAYBACK_ADDR = get_method_addr(AtomSourceEx, c"set_Playback", 1);
        GET_ISINUSE_ADDR = get_method_addr(AtomSourceEx, c"get_IsInUse", 0);
        STOP_ADDR = get_method_addr(AtomSourceEx, c"Stop", 2);
    }
}

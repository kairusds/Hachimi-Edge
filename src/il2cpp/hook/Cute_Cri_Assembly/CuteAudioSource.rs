use crate::{
    core::live_utils::AudioPlayback,
    il2cpp::{
        symbols::{get_method_addr, get_field_from_name},
        types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;

pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_field_object_accessors!(get_sourceList, set_sourceList, _SOURCELIST_FIELD, Il2CppObject);

def_field_value_accessors!(get_usingIndex, set_usingIndex, _USINGINDEX_FIELD, i32);

static mut IS_SAME_PLAYBACK_ID_ADDR: usize = 0;
impl_addr_wrapper_fn!(IsSamePlaybackId, IS_SAME_PLAYBACK_ID_ADDR, bool, this: *mut Il2CppObject, playback: AudioPlayback);

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", CuteAudioSource);

    unsafe {
        CLASS = CuteAudioSource;
        _SOURCELIST_FIELD = get_field_from_name(CuteAudioSource, c"sourceList");
        _USINGINDEX_FIELD = get_field_from_name(CuteAudioSource, c"usingIndex");
        IS_SAME_PLAYBACK_ID_ADDR = get_method_addr(CuteAudioSource, c"IsSamePlaybackId", 1);
    }
}

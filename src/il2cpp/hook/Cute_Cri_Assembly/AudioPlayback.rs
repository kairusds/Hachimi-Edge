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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AudioPlayback_t {
    pub criAtomExPlayback: CriAtomExPlayback_t,
    pub isError: bool,
    pub soundGroup: i32,
    pub is3dSound: bool,
    pub atomSourceListIndex: i32,
    pub cueSheetName: *mut Il2CppString,
    pub cueName: *mut Il2CppString,
    pub cueId: i32,
}

def_method_wrapper_fn!(GetNumPlayedSamples, GET_NUM_PLAYED_SAMPLES_ADDR, bool, this: *mut AudioPlayback_t, num_samples: *mut i64, sampling_rate: *mut i32);

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", AudioPlayback);

    unsafe {
        CLASS = AudioPlayback;
        GET_NUM_PLAYED_SAMPLES_ADDR = get_method_addr(AudioPlayback, c"GetNumPlayedSamples", 2);
    }
}

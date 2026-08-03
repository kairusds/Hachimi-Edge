use crate::{
    windows::free_camera::{self, CameraScene},
    il2cpp::{
        symbols::get_method_addr,
        types::*,
    },
};

use super::{HorseRaceInfoReplay, RaceViewBase};

type OnDestroyFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn OnDestroy(this: *mut Il2CppObject) {
    RaceViewBase::restore_race_disabled_heads(0, true);
    HorseRaceInfoReplay::clear_gate_no_cache();
    free_camera::end_scene(CameraScene::Race);
    get_orig_fn!(OnDestroy, OnDestroyFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", RaceEffectManager);

    let OnDestroy_addr = get_method_addr(RaceEffectManager, c"OnDestroy", 0);
    new_hook!(OnDestroy_addr, OnDestroy);
}

use crate::{
    core::free_camera::{self, CameraScene},
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

use super::{HorseRaceInfoReplay, RaceViewBase};

type NoArgsFn = extern "C" fn(this: *mut Il2CppObject);

extern "C" fn RaceEffectManager_OnDestroy(this: *mut Il2CppObject) {
    RaceViewBase::restore_race_disabled_heads(0, true);
    HorseRaceInfoReplay::clear_gate_no_cache();
    free_camera::end_scene(CameraScene::Race);
    get_orig_fn!(RaceEffectManager_OnDestroy, NoArgsFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(race_effect_manager) = get_class(umamusume, c"Gallop", c"RaceEffectManager") {
        let RaceEffectManager_OnDestroy_addr = get_method_addr(race_effect_manager, c"OnDestroy", 0);
        new_hook!(RaceEffectManager_OnDestroy_addr, RaceEffectManager_OnDestroy);
    }
}

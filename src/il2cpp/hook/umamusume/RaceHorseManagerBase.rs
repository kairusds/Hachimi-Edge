use std::sync::atomic::{AtomicBool, Ordering};

use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static RACE_ACTIVE: AtomicBool = AtomicBool::new(false);
pub fn is_race_active() -> bool {
    RACE_ACTIVE.load(Ordering::Acquire)
}

def_method_wrapper_fn!(GetHorseRaceInfos, GET_HORSE_RACE_INFOS_ADDR, *mut Il2CppArray, this: *mut Il2CppObject);
def_method_wrapper_fn!(GetPlayerHorseIndex, GET_PLAYER_HORSE_INDEX_ADDR, i32, this: *mut Il2CppObject);

type RaceHorseManagerBase_InitFn = extern "C" fn(this: *mut Il2CppObject, raceInfo: *mut Il2CppObject);
extern "C" fn RaceHorseManagerBase_Init(this: *mut Il2CppObject, raceInfo: *mut Il2CppObject) {
    RACE_ACTIVE.store(true, Ordering::Release);
    get_orig_fn!(RaceHorseManagerBase_Init, RaceHorseManagerBase_InitFn)(this, raceInfo);
}

type RaceHorseManagerBase_ReleaseFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn RaceHorseManagerBase_Release(this: *mut Il2CppObject) {
    RACE_ACTIVE.store(false, Ordering::Release);
    get_orig_fn!(RaceHorseManagerBase_Release, RaceHorseManagerBase_ReleaseFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceHorseManagerBase);

    unsafe {
        GET_HORSE_RACE_INFOS_ADDR = get_method_addr(RaceHorseManagerBase, c"GetHorseRaceInfos", 0);
        GET_PLAYER_HORSE_INDEX_ADDR = get_method_addr(RaceHorseManagerBase, c"GetPlayerHorseIndex", 0);
    }

    let Init_addr = get_method_addr(RaceHorseManagerBase, c"Init", 1);
    new_hook!(Init_addr, RaceHorseManagerBase_Init);

    let Release_addr = get_method_addr(RaceHorseManagerBase, c"Release", 0);
    new_hook!(Release_addr, RaceHorseManagerBase_Release);
}

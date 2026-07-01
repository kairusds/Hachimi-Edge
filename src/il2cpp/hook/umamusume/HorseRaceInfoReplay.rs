use std::{
    collections::HashMap,
    sync::Mutex,
};

use once_cell::sync::Lazy;

use crate::{
    core::{free_camera, Hachimi},
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

use super::{HorseData, HorseRaceInfo};

static RACE_INFO_GATE_NO: Lazy<Mutex<HashMap<usize, i32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub(crate) fn clear_gate_no_cache() {
    RACE_INFO_GATE_NO.lock().unwrap().clear();
}

type HorseRaceInfoReplayCtorFn = extern "C" fn(
    this: *mut Il2CppObject,
    data: *mut Il2CppObject,
    reader: *mut Il2CppObject,
);
type GetRunMotionSpeedFn = extern "C" fn(this: *mut Il2CppObject) -> f32;

extern "C" fn HorseRaceInfoReplay_ctor(
    this: *mut Il2CppObject,
    data: *mut Il2CppObject,
    reader: *mut Il2CppObject,
) {
    get_orig_fn!(HorseRaceInfoReplay_ctor, HorseRaceInfoReplayCtorFn)(this, data, reader);

    if data.is_null() {
        return;
    }

    let Some(gate_no) = HorseData::get_GateNo(data) else {
        return;
    };
    RACE_INFO_GATE_NO.lock().unwrap().insert(this as usize, gate_no - 1);
}

extern "C" fn HorseRaceInfoReplay_get_RunMotionSpeed(this: *mut Il2CppObject) -> f32 {
    let result = get_orig_fn!(HorseRaceInfoReplay_get_RunMotionSpeed, GetRunMotionSpeedFn)(this);

    if !Hachimi::instance().config.load().free_camera.enabled {
        return result;
    }

    let gate_no = RACE_INFO_GATE_NO
        .lock()
        .unwrap()
        .get(&(this as usize))
        .copied()
        .unwrap_or(-1);
    if gate_no < 0 {
        return result;
    }

    let Some((pos, rot)) = HorseRaceInfo::position_and_rotation_on_lane(this) else {
        return result;
    };
    free_camera::update_race_target(gate_no, pos, rot);
    result
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(horse_race_info_replay) = get_class(umamusume, c"Gallop", c"HorseRaceInfoReplay") {
        let HorseRaceInfoReplay_ctor_addr = get_method_addr(horse_race_info_replay, c".ctor", 2);
        let HorseRaceInfoReplay_get_RunMotionSpeed_addr =
            get_method_addr(horse_race_info_replay, c"get_RunMotionSpeed", 0);
        new_hook!(HorseRaceInfoReplay_ctor_addr, HorseRaceInfoReplay_ctor);
        new_hook!(HorseRaceInfoReplay_get_RunMotionSpeed_addr, HorseRaceInfoReplay_get_RunMotionSpeed);
    }
}

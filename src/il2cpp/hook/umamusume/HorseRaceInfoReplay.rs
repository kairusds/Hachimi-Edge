use std::{
    collections::HashMap,
    sync::Mutex,
};

use once_cell::sync::Lazy;

use crate::{
    core::Hachimi,
    il2cpp::{
        symbols::get_method_addr,
        types::*,
    },
    windows::free_camera,
};

use super::{HorseData, HorseRaceInfo};

static RACE_INFO_GATE_NO: Lazy<Mutex<HashMap<usize, i32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn clear_gate_no_cache() {
    RACE_INFO_GATE_NO.lock().unwrap().clear();
}

type HorseRaceInfoReplayCtorFn = extern "C" fn(
    this: *mut Il2CppObject,
    data: *mut Il2CppObject,
    reader: *mut Il2CppObject,
);
extern "C" fn ctor(
    this: *mut Il2CppObject,
    data: *mut Il2CppObject,
    reader: *mut Il2CppObject,
) {
    get_orig_fn!(ctor, HorseRaceInfoReplayCtorFn)(this, data, reader);

    if data.is_null() {
        return;
    }

    let gate_no = HorseData::get_GateNo(data);
    RACE_INFO_GATE_NO.lock().unwrap().insert(this as usize, gate_no - 1);
}

type get_RunMotionSpeedFn = extern "C" fn(this: *mut Il2CppObject) -> f32;
extern "C" fn get_RunMotionSpeed(this: *mut Il2CppObject) -> f32 {
    let result = get_orig_fn!(get_RunMotionSpeed, get_RunMotionSpeedFn)(this);

    if !Hachimi::instance().config.load().windows.free_camera.enabled {
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

    let pos = HorseRaceInfo::get__position(this);
    let rot = HorseRaceInfo::get__rotationOnLane(this);
    free_camera::update_race_target(gate_no, pos, rot);
    result
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, HorseRaceInfoReplay);

    let ctor_addr = get_method_addr(HorseRaceInfoReplay, c".ctor", 2);
    new_hook!(ctor_addr, ctor);

    let get_RunMotionSpeed_addr = get_method_addr(HorseRaceInfoReplay, c"get_RunMotionSpeed", 0);
    new_hook!(get_RunMotionSpeed_addr, get_RunMotionSpeed);
}

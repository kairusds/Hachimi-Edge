use crate::il2cpp::{
    symbols::{get_field_from_name, get_method_addr},
    types::*,
};

use super::TemptationMode;

#[cfg(target_os = "windows")]
use std::{
    collections::HashMap,
    sync::Mutex,
};

#[cfg(target_os = "windows")]
use once_cell::sync::Lazy;

#[cfg(target_os = "windows")]
use crate::{
    core::Hachimi,
    windows::free_camera,
};

#[cfg(target_os = "windows")]
use super::{HorseData, HorseRaceInfo};

#[cfg(target_os = "windows")]
static RACE_INFO_GATE_NO: Lazy<Mutex<HashMap<usize, i32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(target_os = "windows")]
pub fn clear_gate_no_cache() {
    RACE_INFO_GATE_NO.lock().unwrap().clear();
}

#[cfg(target_os = "windows")]
type HorseRaceInfoReplayCtorFn = extern "C" fn(
    this: *mut Il2CppObject,
    data: *mut Il2CppObject,
    reader: *mut Il2CppObject,
);
#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
type get_RunMotionSpeedFn = extern "C" fn(this: *mut Il2CppObject) -> f32;
#[cfg(target_os = "windows")]
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

def_field_value_accessors!(get__temptationMode, set__temptationMode, TEMPTATION_MODE_FIELD, TemptationMode);
def_field_value_accessors!(get__temptationCount, set__temptationCount, TEMPTATION_COUNT_FIELD, i32);
def_field_value_accessors!(get__lastSpurtStartDistance, set__lastSpurtStartDistance, LAST_SPURT_START_DISTANCE_FIELD, f32);

def_method_wrapper_fn!(get_IsLastSpurt, GET_IS_LAST_SPURT_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_FinishOrder, GET_FINISH_ORDER_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_FinishTimeScaled, GET_FINISH_TIME_SCALED_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_FinishTimeDiffFromPrevHorse, GET_FINISH_TIME_DIFF_ADDR, f32, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, HorseRaceInfoReplay);

    unsafe {
        TEMPTATION_MODE_FIELD = get_field_from_name(HorseRaceInfoReplay, c"_temptationMode");
        TEMPTATION_COUNT_FIELD = get_field_from_name(HorseRaceInfoReplay, c"_temptationCount");
        LAST_SPURT_START_DISTANCE_FIELD = get_field_from_name(HorseRaceInfoReplay, c"_lastSpurtStartDistance");
        GET_IS_LAST_SPURT_ADDR = get_method_addr(HorseRaceInfoReplay, c"get_IsLastSpurt", 0);
        GET_FINISH_ORDER_ADDR = get_method_addr(HorseRaceInfoReplay, c"get_FinishOrder", 0);
        GET_FINISH_TIME_SCALED_ADDR = get_method_addr(HorseRaceInfoReplay, c"get_FinishTimeScaled", 0);
        GET_FINISH_TIME_DIFF_ADDR = get_method_addr(HorseRaceInfoReplay, c"get_FinishTimeDiffFromPrevHorse", 0);
    }

    #[cfg(target_os = "windows")]
    {
        let ctor_addr = get_method_addr(HorseRaceInfoReplay, c".ctor", 2);
        new_hook!(ctor_addr, ctor);

        let get_RunMotionSpeed_addr = get_method_addr(HorseRaceInfoReplay, c"get_RunMotionSpeed", 0);
        new_hook!(get_RunMotionSpeed_addr, get_RunMotionSpeed);
    }
}

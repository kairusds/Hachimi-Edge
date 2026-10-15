use crate::il2cpp::{
    symbols::{get_field_from_name, get_method_addr},
    types::*
};

use super::{RaceManager, RaceHorseManagerReplay};

def_field_object_accessors!(get__simData, set__simData, SIM_DATA_FIELD, Il2CppObject);
def_field_value_accessors!(get__curTime, set__curTime, CUR_TIME_FIELD, f32);

def_method_wrapper_fn!(GetLastFrameTime, GET_LAST_FRAME_TIME_ADDR, f32, this: *mut Il2CppObject);

pub fn replay_cur_time(race_manager: *mut Il2CppObject) -> Option<f32> {
    let horse_manager = RaceManager::get__horseManager(race_manager);
    if horse_manager.is_null() { return None; }
    if !RaceHorseManagerReplay::is_replay_manager(horse_manager) { return None; }

    let reader = RaceHorseManagerReplay::get__reader(horse_manager);
    if reader.is_null() { return None; }

    let cur_time = get__curTime(reader);
    if cur_time.is_finite() && cur_time >= 0.0 { Some(cur_time) } else { None }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceSimulateReader);

    unsafe {
        SIM_DATA_FIELD = get_field_from_name(RaceSimulateReader, c"_simData");
        CUR_TIME_FIELD = get_field_from_name(RaceSimulateReader, c"_curTime");
        GET_LAST_FRAME_TIME_ADDR = get_method_addr(RaceSimulateReader, c"GetLastFrameTime", 0);
    }
}

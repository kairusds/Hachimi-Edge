use crate::il2cpp::{symbols::get_method_addr, types::*};

def_method_wrapper_fn!(get_RaceType, GET_RACETYPE_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(set_RaceType, SET_RACETYPE_ADDR, (), this: *mut Il2CppObject, value: i32);
def_method_wrapper_fn!(get_CourseOnlyDistance, GET_COURSE_ONLY_DISTANCE_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RunUpDistance, GET_RUN_UP_DISTANCE_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_PhaseMiddleStartDistance, GET_PHASE_MIDDLE_START_DISTANCE_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_PhaseEndStartDistance, GET_PHASE_END_START_DISTANCE_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_PhaseLastStartDistance, GET_PHASE_LAST_START_DISTANCE_ADDR, f32, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceInfo);

    unsafe {
        GET_RACETYPE_ADDR = get_method_addr(RaceInfo, c"get_RaceType", 0);
        SET_RACETYPE_ADDR = get_method_addr(RaceInfo, c"set_RaceType", 1);
        GET_COURSE_ONLY_DISTANCE_ADDR = get_method_addr(RaceInfo, c"get_CourseOnlyDistance", 0);
        GET_RUN_UP_DISTANCE_ADDR = get_method_addr(RaceInfo, c"get_RunUpDistance", 0);
        GET_PHASE_MIDDLE_START_DISTANCE_ADDR = get_method_addr(RaceInfo, c"get_PhaseMiddleStartDistance", 0);
        GET_PHASE_END_START_DISTANCE_ADDR = get_method_addr(RaceInfo, c"get_PhaseEndStartDistance", 0);
        GET_PHASE_LAST_START_DISTANCE_ADDR = get_method_addr(RaceInfo, c"get_PhaseLastStartDistance", 0);
    }
}
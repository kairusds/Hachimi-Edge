use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

def_method_wrapper_fn!(get_startDistance, GET_STARTDISTANCE_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_finishDistance, GET_FINISHDISTANCE_ADDR, f32, this: *mut Il2CppObject);

pub fn init(RaceSimulateEventData: *mut Il2CppClass) {
    find_nested_class_or_return!(RaceSimulateEventData, DistanceData);

    unsafe {
        GET_STARTDISTANCE_ADDR = get_method_addr(DistanceData, c"get_startDistance", 0);
        GET_FINISHDISTANCE_ADDR = get_method_addr(DistanceData, c"get_finishDistance", 0);
    }
}

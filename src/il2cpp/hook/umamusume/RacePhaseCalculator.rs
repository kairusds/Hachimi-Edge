use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
};

def_method_wrapper_fn!(get_PhaseMiddleStartDistance, GET_PHASE_MIDDLE_START_DISTANCE_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_PhaseEndStartDistance, GET_PHASE_END_START_DISTANCE_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_PhaseLastStartDistance, GET_PHASE_LAST_START_DISTANCE_ADDR, f32, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Global {
        return;
    }

    get_class_or_return!(umamusume, Gallop, RacePhaseCalculator);

    unsafe {
        GET_PHASE_MIDDLE_START_DISTANCE_ADDR = get_method_addr(RacePhaseCalculator, c"get_PhaseMiddleStartDistance", 0);
        GET_PHASE_END_START_DISTANCE_ADDR = get_method_addr(RacePhaseCalculator, c"get_PhaseEndStartDistance", 0);
        GET_PHASE_LAST_START_DISTANCE_ADDR = get_method_addr(RacePhaseCalculator, c"get_PhaseLastStartDistance", 0);
    }
}

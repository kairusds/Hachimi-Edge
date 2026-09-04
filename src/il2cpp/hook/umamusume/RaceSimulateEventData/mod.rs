use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        hook::umamusume::SimulateEventType,
        symbols::get_field_from_name,
        types::*
    }
};

pub mod DistanceData;

def_field_object_accessors!(get_distanceData, set_distanceData, DISTANCE_DATA_FIELD, Il2CppObject);
def_field_value_accessors!(get_type, set_type, TYPE_FIELD, SimulateEventType);
def_field_object_accessors!(get_param, set_param, PARAM_FIELD, Il2CppArray);

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, StandaloneSimulator, RaceSimulateEventData);

    unsafe {
        DISTANCE_DATA_FIELD = get_field_from_name(RaceSimulateEventData, c"distanceData");
        TYPE_FIELD = get_field_from_name(RaceSimulateEventData, c"type");
        PARAM_FIELD = get_field_from_name(RaceSimulateEventData, c"param");
    }

    DistanceData::init(RaceSimulateEventData);
}

use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        symbols::get_field_from_name,
        types::*
    }
};

def_field_value_accessors!(get_Time, set_Time, TIME_FIELD, f32);
def_field_object_accessors!(get_HorseDataArray, set_HorseDataArray, HORSE_DATA_ARRAY_FIELD, Il2CppArray);

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, StandaloneSimulator, RaceSimulateFrameData);

    unsafe {
        TIME_FIELD = get_field_from_name(RaceSimulateFrameData, c"Time");
        HORSE_DATA_ARRAY_FIELD = get_field_from_name(RaceSimulateFrameData, c"HorseDataArray");
    }
}

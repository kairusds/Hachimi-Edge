use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        symbols::get_field_from_name,
        types::*
    }
};

def_field_object_accessors!(get__simData, set__simData, SIM_DATA_FIELD, Il2CppObject);
def_field_value_accessors!(get__curTime, set__curTime, CUR_TIME_FIELD, f32);

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, RaceSimulateReader);

    unsafe {
        SIM_DATA_FIELD = get_field_from_name(RaceSimulateReader, c"_simData");
        CUR_TIME_FIELD = get_field_from_name(RaceSimulateReader, c"_curTime");
    }
}

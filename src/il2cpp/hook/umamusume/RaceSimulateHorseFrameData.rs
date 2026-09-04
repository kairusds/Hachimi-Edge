use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        symbols::get_field_from_name,
        types::*
    }
};

def_field_value_accessors!(get_Speed, set_Speed, SPEED_FIELD, f32);
def_field_value_accessors!(get_Hp, set_Hp, HP_FIELD, f32);

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, StandaloneSimulator, RaceSimulateHorseFrameData);

    unsafe {
        SPEED_FIELD = get_field_from_name(RaceSimulateHorseFrameData, c"Speed");
        HP_FIELD = get_field_from_name(RaceSimulateHorseFrameData, c"Hp");
    }
}

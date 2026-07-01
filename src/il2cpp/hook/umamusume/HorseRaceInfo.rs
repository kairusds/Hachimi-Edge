use std::ptr::null_mut;

use crate::il2cpp::{
    symbols::{get_class, get_field_from_name, get_field_value},
    types::*,
};

static mut POSITION_FIELD: *mut FieldInfo = null_mut();
static mut ROTATION_ON_LANE_FIELD: *mut FieldInfo = null_mut();

pub fn position_and_rotation_on_lane(this: *mut Il2CppObject) -> Option<(Vector3_t, Quaternion_t)> {
    if unsafe { POSITION_FIELD.is_null() || ROTATION_ON_LANE_FIELD.is_null() } {
        return None;
    }

    let pos: Vector3_t = get_field_value(this, unsafe { POSITION_FIELD });
    let rot: Quaternion_t = get_field_value(this, unsafe { ROTATION_ON_LANE_FIELD });
    Some((pos, rot))
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(horse_race_info) = get_class(umamusume, c"Gallop", c"HorseRaceInfo") {
        unsafe {
            POSITION_FIELD = get_field_from_name(horse_race_info, c"_position");
            ROTATION_ON_LANE_FIELD = get_field_from_name(horse_race_info, c"_rotationOnLane");
        }
    }
}

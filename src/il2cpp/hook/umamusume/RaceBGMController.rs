use crate::il2cpp::{
    api::il2cpp_class_is_assignable_from,
    ext::Il2CppObjectExt,
    symbols::get_field_from_name,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn is_bgm_controller(obj: *mut Il2CppObject) -> bool {
    if class().is_null() || obj.is_null() {
        return false;
    }

    let obj_class = unsafe { (*obj).klass() };
    !obj_class.is_null() && il2cpp_class_is_assignable_from(class(), obj_class)
}

def_field_value_accessors!(get_isStoppedFirstBGM, set_isStoppedFirstBGM, IS_STOPPED_FIRST_BGM_FIELD, bool);
def_field_value_accessors!(get_isRequestFirstBGM, set_isRequestFirstBGM, IS_REQUEST_FIRST_BGM_FIELD, bool);
def_field_value_accessors!(get_firstBGMDelayTime, set_firstBGMDelayTime, FIRST_BGM_DELAY_TIME_FIELD, f32);
def_field_value_accessors!(get_firstBgmStopTime, set_firstBgmStopTime, FIRST_BGM_STOP_TIME_FIELD, f32);
def_field_value_accessors!(get_isPlayedSecondBGM, set_isPlayedSecondBGM, IS_PLAYED_SECOND_BGM_FIELD, bool);
def_field_value_accessors!(get_secondBgmStartTime, set_secondBgmStartTime, SECOND_BGM_START_TIME_FIELD, f32);
def_field_object_accessors!(get_firstBgmCueName, set_firstBgmCueName, FIRST_BGM_CUE_NAME_FIELD, Il2CppString);
def_field_object_accessors!(get_secondBgmCueName, set_secondBgmCueName, SECOND_BGM_CUE_NAME_FIELD, Il2CppString);
def_field_value_accessors!(get_firstTriggerBgmPlayStartTime, set_firstTriggerBgmPlayStartTime, FIRST_TRIGGER_BGM_PLAY_START_TIME_FIELD, f32);
def_field_value_accessors!(get_isPlayedFirstTriggerBgm, set_isPlayedFirstTriggerBgm, IS_PLAYED_FIRST_TRIGGER_BGM_FIELD, bool);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceBGMController);

    unsafe {
        CLASS = RaceBGMController;
        IS_STOPPED_FIRST_BGM_FIELD = get_field_from_name(RaceBGMController, c"_isStoppedFirstBGM");
        IS_REQUEST_FIRST_BGM_FIELD = get_field_from_name(RaceBGMController, c"_isRequestFirstBGM");
        FIRST_BGM_DELAY_TIME_FIELD = get_field_from_name(RaceBGMController, c"_firstBGMDelayTime");
        FIRST_BGM_STOP_TIME_FIELD = get_field_from_name(RaceBGMController, c"_firstBgmStopTime");
        IS_PLAYED_SECOND_BGM_FIELD = get_field_from_name(RaceBGMController, c"_isPlayedSecondBGM");
        SECOND_BGM_START_TIME_FIELD = get_field_from_name(RaceBGMController, c"_secondBgmStartTime");
        FIRST_BGM_CUE_NAME_FIELD = get_field_from_name(RaceBGMController, c"_firstBgmCueName");
        SECOND_BGM_CUE_NAME_FIELD = get_field_from_name(RaceBGMController, c"_secondBgmCueName");
        FIRST_TRIGGER_BGM_PLAY_START_TIME_FIELD = get_field_from_name(RaceBGMController, c"_firstTriggerBgmPlayStartTime");
        IS_PLAYED_FIRST_TRIGGER_BGM_FIELD = get_field_from_name(RaceBGMController, c"_isPlayedFirstTriggerBgm");
    }
}

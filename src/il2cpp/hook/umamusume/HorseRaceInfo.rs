use crate::il2cpp::{
    symbols::{get_field_from_name, get_method_addr},
    types::*,
};

use super::RaceDefine;

def_field_value_accessors!(get__position, set__position, POSITION_FIELD, Vector3_t);
def_field_value_accessors!(get__rotationOnLane, set__rotationOnLane, ROTATION_ON_LANE_FIELD, Quaternion_t);

def_field_value_accessors!(get__lastSpeed, set__lastSpeed, LAST_SPEED_FIELD, f32);
def_field_value_accessors!(get__hp, set__hp, HP_FIELD, f32);
def_field_value_accessors!(get__maxHp, set__maxHp, MAX_HP_FIELD, f32);
def_field_object_accessors!(get__skillManager, set__skillManager, SKILL_MANAGER_FIELD, Il2CppObject);
def_field_value_accessors!(get__phase, set__phase, PHASE_FIELD, RaceDefine::HorsePhase);
def_field_value_accessors!(get__minSpeed, set__minSpeed, MIN_SPEED_FIELD, f32);
def_field_value_accessors!(get__maxSpeedInRace, set__maxSpeedInRace, MAX_SPEED_IN_RACE_FIELD, f32);
def_field_value_accessors!(get__lastSelfSpeed, set__lastSelfSpeed, LAST_SELF_SPEED_FIELD, f32);
def_field_value_accessors!(get__laneDistance, set__laneDistance, LANE_DISTANCE_FIELD, f32);
def_field_value_accessors!(get__distance, set__distance, DISTANCE_FIELD, f32);
def_field_object_accessors!(get__horseRaceAI, set__horseRaceAI, HORSE_RACE_AI_FIELD, Il2CppObject);

def_method_wrapper_fn!(get_CharaName, GET_CHARA_NAME_ADDR, *mut Il2CppString, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_HorseData, GET_HORSE_DATA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_Lane, GET_LANE_ADDR, RaceDefine::LaneType, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_Motivation, GET_MOTIVATION_ADDR, RaceDefine::Motivation, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_MotivationCoef, GET_MOTIVATION_COEF_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RawSpeed, GET_RAW_SPEED_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RawStamina, GET_RAW_STAMINA_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RawPow, GET_RAW_POW_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RawGuts, GET_RAW_GUTS_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RawWiz, GET_RAW_WIZ_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_DelayTime, GET_DELAYTIME_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsGoodStart, GET_ISGOODSTART_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsBadStart, GET_ISBADSTART_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsStartDash, GET_ISSTARTDASH_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsClog, GET_ISCLOG_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsCompeteFight, GET_ISCOMPETEFIGHT_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_CompeteFightCount, GET_COMPETEFIGHTCOUNT_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsCompeteTop, GET_ISCOMPETETOP_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_CompeteTopCount, GET_COMPETETOPCOUNT_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_CompeteTopRemainTime, GET_COMPETETOPREMAINTIME_ADDR, f32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_CurOrder, GET_CURORDER_ADDR, i32, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_PrevOrder, GET_PREVORDER_ADDR, i32, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, HorseRaceInfo);

    unsafe {
        // fields
        POSITION_FIELD = get_field_from_name(HorseRaceInfo, c"_position");
        ROTATION_ON_LANE_FIELD = get_field_from_name(HorseRaceInfo, c"_rotationOnLane");
        LAST_SPEED_FIELD = get_field_from_name(HorseRaceInfo, c"_lastSpeed");
        HP_FIELD = get_field_from_name(HorseRaceInfo, c"_hp");
        MAX_HP_FIELD = get_field_from_name(HorseRaceInfo, c"_maxHp");
        SKILL_MANAGER_FIELD = get_field_from_name(HorseRaceInfo, c"_skillManager");
        PHASE_FIELD = get_field_from_name(HorseRaceInfo, c"_phase");
        MIN_SPEED_FIELD = get_field_from_name(HorseRaceInfo, c"_minSpeed");
        MAX_SPEED_IN_RACE_FIELD = get_field_from_name(HorseRaceInfo, c"_maxSpeedInRace");
        LAST_SELF_SPEED_FIELD = get_field_from_name(HorseRaceInfo, c"_lastSelfSpeed");
        LANE_DISTANCE_FIELD = get_field_from_name(HorseRaceInfo, c"_laneDistance");
        DISTANCE_FIELD = get_field_from_name(HorseRaceInfo, c"_distance");
        HORSE_RACE_AI_FIELD = get_field_from_name(HorseRaceInfo, c"_horseRaceAI");

        // methods
        GET_CHARA_NAME_ADDR = get_method_addr(HorseRaceInfo, c"get_CharaName", 0);
        GET_HORSE_DATA_ADDR = get_method_addr(HorseRaceInfo, c"get_HorseData", 0);
        GET_LANE_ADDR = get_method_addr(HorseRaceInfo, c"get_Lane", 0);
        GET_MOTIVATION_ADDR = get_method_addr(HorseRaceInfo, c"get_Motivation", 0);
        GET_MOTIVATION_COEF_ADDR = get_method_addr(HorseRaceInfo, c"get_MotivationCoef", 0);
        GET_RAW_SPEED_ADDR = get_method_addr(HorseRaceInfo, c"get_RawSpeed", 0);
        GET_RAW_STAMINA_ADDR = get_method_addr(HorseRaceInfo, c"get_RawStamina", 0);
        GET_RAW_POW_ADDR = get_method_addr(HorseRaceInfo, c"get_RawPow", 0);
        GET_RAW_GUTS_ADDR = get_method_addr(HorseRaceInfo, c"get_RawGuts", 0);
        GET_RAW_WIZ_ADDR = get_method_addr(HorseRaceInfo, c"get_RawWiz", 0);
        GET_DELAYTIME_ADDR = get_method_addr(HorseRaceInfo, c"get_DelayTime", 0);
        GET_ISGOODSTART_ADDR = get_method_addr(HorseRaceInfo, c"get_IsGoodStart", 0);
        GET_ISBADSTART_ADDR = get_method_addr(HorseRaceInfo, c"get_IsBadStart", 0);
        GET_ISSTARTDASH_ADDR = get_method_addr(HorseRaceInfo, c"get_IsStartDash", 0);
        GET_ISCLOG_ADDR = get_method_addr(HorseRaceInfo, c"get_IsClog", 0);
        GET_ISCOMPETEFIGHT_ADDR = get_method_addr(HorseRaceInfo, c"get_IsCompeteFight", 0);
        GET_COMPETEFIGHTCOUNT_ADDR = get_method_addr(HorseRaceInfo, c"get_CompeteFightCount", 0);
        GET_ISCOMPETETOP_ADDR = get_method_addr(HorseRaceInfo, c"get_IsCompeteTop", 0);
        GET_COMPETETOPCOUNT_ADDR = get_method_addr(HorseRaceInfo, c"get_CompeteTopCount", 0);
        GET_COMPETETOPREMAINTIME_ADDR = get_method_addr(HorseRaceInfo, c"get_CompeteTopRemainTime", 0);
        GET_CURORDER_ADDR = get_method_addr(HorseRaceInfo, c"get_CurOrder", 0);
        GET_PREVORDER_ADDR = get_method_addr(HorseRaceInfo, c"get_PrevOrder", 0);
    }
}

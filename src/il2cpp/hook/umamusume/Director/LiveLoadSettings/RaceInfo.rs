use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GET_CHAMPIONSMEETINGRESOURCEID_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_ChampionsMeetingResourceId, GET_CHAMPIONSMEETINGRESOURCEID_ADDR, i32, this: *mut Il2CppObject);

static mut SET_CHAMPIONSMEETINGRESOURCEID_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_ChampionsMeetingResourceId, SET_CHAMPIONSMEETINGRESOURCEID_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut SET_DATEYEAR_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_DateYear, SET_DATEYEAR_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut SET_CHARACTERNAMEARRAY_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_CharacterNameArray, SET_CHARACTERNAMEARRAY_ADDR, (), this: *mut Il2CppObject, value: *mut Il2CppArray);

static mut SET_TRAINERNAMEARRAY_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_TrainerNameArray, SET_TRAINERNAMEARRAY_ADDR, (), this: *mut Il2CppObject, value: *mut Il2CppArray);

static mut SET_CHARACTERNAMEARRAYFORCHAMPIONSTEXT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_CharacterNameArrayForChampionsText, SET_CHARACTERNAMEARRAYFORCHAMPIONSTEXT_ADDR, (), this: *mut Il2CppObject, value: *mut Il2CppArray);

static mut SET_TRAINERNAMEARRAYFORCHAMPIONSTEXT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_TrainerNameArrayForChampionsText, SET_TRAINERNAMEARRAYFORCHAMPIONSTEXT_ADDR, (), this: *mut Il2CppObject, value: *mut Il2CppArray);

pub fn init(LiveLoadSettings: *mut Il2CppClass) {
    find_nested_class_or_return!(LiveLoadSettings, RaceInfo);

    unsafe {
        CLASS = RaceInfo;
        GET_CHAMPIONSMEETINGRESOURCEID_ADDR = get_method_addr(RaceInfo, c"get_ChampionsMeetingResourceId", 0);
        SET_CHAMPIONSMEETINGRESOURCEID_ADDR = get_method_addr(RaceInfo, c"set_ChampionsMeetingResourceId", 1);
        SET_DATEYEAR_ADDR = get_method_addr(RaceInfo, c"set_DateYear", 1);
        SET_CHARACTERNAMEARRAY_ADDR = get_method_addr(RaceInfo, c"set_CharacterNameArray", 1);
        SET_TRAINERNAMEARRAY_ADDR = get_method_addr(RaceInfo, c"set_TrainerNameArray", 1);
        SET_CHARACTERNAMEARRAYFORCHAMPIONSTEXT_ADDR = get_method_addr(RaceInfo, c"set_CharacterNameArrayForChampionsText", 1);
        SET_TRAINERNAMEARRAYFORCHAMPIONSTEXT_ADDR = get_method_addr(RaceInfo, c"set_TrainerNameArrayForChampionsText", 1);
    }
}

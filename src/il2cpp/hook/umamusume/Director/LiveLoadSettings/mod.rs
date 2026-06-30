use crate::il2cpp::{
    symbols::{get_method_addr},
    types::*
};

pub mod CharacterInfo;
pub mod RaceInfo;

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GET_MUSICID_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_MusicId, GET_MUSICID_ADDR, i32, this: *mut Il2CppObject);

static mut GET_RACEINFO_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_raceInfo, GET_RACEINFO_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_CHARACTERINFOLIST_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_CharacterInfoList, GET_CHARACTERINFOLIST_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(Director: *mut Il2CppClass) {
    find_nested_class_or_return!(Director, LiveLoadSettings);

    CharacterInfo::init(LiveLoadSettings);
    RaceInfo::init(LiveLoadSettings);

    unsafe {
        CLASS = LiveLoadSettings;
        GET_MUSICID_ADDR = get_method_addr(LiveLoadSettings, c"get_MusicId", 0);
        GET_RACEINFO_ADDR = get_method_addr(LiveLoadSettings, c"get_raceInfo", 0);
        GET_CHARACTERINFOLIST_ADDR = get_method_addr(LiveLoadSettings, c"get_CharacterInfoList", 0);
    }
}

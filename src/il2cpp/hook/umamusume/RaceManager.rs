use crate::il2cpp::{
    symbols::{get_field_from_name, get_method_addr, SingletonLike},
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

def_field_object_accessors!(get get__horseManager, _HORSEMANAGER_FIELD, Il2CppObject);

def_method_wrapper_fn!(get_RaceInfo, GET_RACE_INFO_ADDR, *mut Il2CppObject,);
def_method_wrapper_fn!(get_RaceSound, GET_RACE_SOUND_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RaceView, GET_RACE_VIEW_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RaceMainView, GET_RACEMAINVIEW_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceManager);

    unsafe {
        CLASS = RaceManager;
        _HORSEMANAGER_FIELD = get_field_from_name(RaceManager, c"_horseManager");
        GET_RACE_INFO_ADDR = get_method_addr(RaceManager, c"get_RaceInfo", 0);
        GET_RACE_SOUND_ADDR = get_method_addr(RaceManager, c"get_RaceSound", 0);
        GET_RACE_VIEW_ADDR = get_method_addr(RaceManager, c"get_RaceView", 0);
        GET_RACEMAINVIEW_ADDR = get_method_addr(RaceManager, c"get_RaceMainView", 0);
    }
}

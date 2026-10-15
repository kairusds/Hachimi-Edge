use crate::il2cpp::{
    symbols::get_field_from_name,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_field_object_accessors!(get get__raceUI, _RACEUI_FIELD, Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceMainViewController);

    unsafe {
        CLASS = RaceMainViewController;
        _RACEUI_FIELD = get_field_from_name(RaceMainViewController, c"_raceUI");
    }
}

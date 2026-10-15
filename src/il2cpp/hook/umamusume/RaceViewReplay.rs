use crate::il2cpp::{
    symbols::get_field_from_name,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_field_value_accessors!(get_lastSpurtProcessed, set_lastSpurtProcessed, LAST_SPURT_PROCESSED_FIELD, bool);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceViewReplay);

    unsafe {
        CLASS = RaceViewReplay;
        LAST_SPURT_PROCESSED_FIELD = get_field_from_name(RaceViewReplay, c"_lastSpurtProcessed");
    }
}

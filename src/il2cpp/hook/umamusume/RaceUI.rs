use crate::il2cpp::{symbols::get_field_from_name, types::*};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_field_object_accessors!(get get__minimap, MINIMAP_FIELD, Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceUI);

    unsafe {
        CLASS = RaceUI;
        MINIMAP_FIELD = get_field_from_name(RaceUI, c"_minimap");
    }
}

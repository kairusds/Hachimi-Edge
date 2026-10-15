use crate::il2cpp::{symbols::get_field_from_name, types::*};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_field_value_accessors!(set set_hasMiniMapShown, HASMINIMAPSHOWN_FIELD, bool);
def_field_value_accessors!(set set_hasMiniMapHidden, HASMINIMAPHIDDEN_FIELD, bool);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceUIMiniMap);

    unsafe {
        CLASS = RaceUIMiniMap;
        HASMINIMAPSHOWN_FIELD = get_field_from_name(RaceUIMiniMap, c"_hasMiniMapShown");
        HASMINIMAPHIDDEN_FIELD = get_field_from_name(RaceUIMiniMap, c"_hasMiniMapHidden");
    }
}

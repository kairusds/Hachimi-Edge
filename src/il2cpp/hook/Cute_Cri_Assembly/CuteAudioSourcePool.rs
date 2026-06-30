use crate::il2cpp::{
    symbols::get_field_from_name,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;

pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_field_object_accessors!(get_sourceList, set_sourceList, _SOURCELIST_FIELD, Il2CppObject);

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", CuteAudioSourcePool);

    unsafe {
        CLASS = CuteAudioSourcePool;
        _SOURCELIST_FIELD = get_field_from_name(CuteAudioSourcePool, c"sourceList");
    }
}

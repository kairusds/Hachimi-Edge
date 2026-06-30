use crate::il2cpp::{
    symbols::get_field_from_name,
    types::*
};

def_field_object_accessors!(get_pool, set_pool, _POOL_FIELD, Il2CppObject);

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", AudioControllerBase);

    unsafe {
        _POOL_FIELD = get_field_from_name(AudioControllerBase, c"pool");
    }
}

use std::ptr::null_mut;

use crate::il2cpp::{
    ext::Il2CppObjectExt,
    symbols::{get_class, get_field_from_name, set_field_value},
    types::*,
};

static mut CLASS: *mut Il2CppClass = null_mut();
static mut IS_ENABLE_DOF_FIELD: *mut FieldInfo = null_mut();

pub fn disable_if_dof(update_info: *mut Il2CppObject) {
    if update_info.is_null() {
        return;
    }

    unsafe {
        if !CLASS.is_null() &&
            !IS_ENABLE_DOF_FIELD.is_null() &&
            (*update_info).klass() == CLASS
        {
            set_field_value(update_info, IS_ENABLE_DOF_FIELD, &false);
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(post_effect_dof) = get_class(umamusume, c"Gallop.Live.Cutt", c"PostEffectUpdateInfo_DOF") {
        unsafe {
            CLASS = post_effect_dof;
            IS_ENABLE_DOF_FIELD = get_field_from_name(post_effect_dof, c"IsEnableDOF");
        }
    }
}

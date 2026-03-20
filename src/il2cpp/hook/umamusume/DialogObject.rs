use crate::{
    il2cpp::{
        symbols::{get_field_from_name, get_field_object_value},
        types::*
    }
};

static mut _BASERECTTRANSFORM_FIELD: *mut FieldInfo = 0 as _;
pub fn get__baseRectTransform(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { _BASERECTTRANSFORM_FIELD })
}

static mut _ROOTRECTTRANSFORM_FIELD: *mut FieldInfo = 0 as _;
pub fn get__rootRectTransform(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { _ROOTRECTTRANSFORM_FIELD })
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DialogObject);

    unsafe {
        _BASERECTTRANSFORM_FIELD = get_field_from_name(DialogObject, c"_baseRectTransform");
        _ROOTRECTTRANSFORM_FIELD = get_field_from_name(DialogObject, c"_rootRectTransform");
    }
}

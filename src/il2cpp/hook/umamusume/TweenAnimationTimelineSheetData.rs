use std::ptr::null_mut;

use crate::il2cpp::{symbols::{get_field_from_name, get_field_object_value}, types::*};

static mut A2UPREFAB_FIELD: *mut FieldInfo = null_mut();
pub fn get_A2UPrefab(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { A2UPREFAB_FIELD })
}

static mut NAME_FIELD: *mut FieldInfo = null_mut();
pub fn get_Name(this: *mut Il2CppObject) -> *mut Il2CppString {
    get_field_object_value(this, unsafe { NAME_FIELD })
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TweenAnimationTimelineSheetData);

    unsafe {
        A2UPREFAB_FIELD = get_field_from_name(TweenAnimationTimelineSheetData, c"A2UPrefab");
        NAME_FIELD = get_field_from_name(TweenAnimationTimelineSheetData, c"Name");
    }
}
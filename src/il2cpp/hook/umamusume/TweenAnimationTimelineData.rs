use crate::il2cpp::{symbols::{get_field_from_name, get_field_object_value}, types::*};

static mut SHEETDATALIST_FIELD: *mut FieldInfo = 0 as _;
pub fn get_SheetDataList(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { SHEETDATALIST_FIELD })
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TweenAnimationTimelineData);

    unsafe {
        SHEETDATALIST_FIELD = get_field_from_name(TweenAnimationTimelineData, c"SheetDataList");
    }
}
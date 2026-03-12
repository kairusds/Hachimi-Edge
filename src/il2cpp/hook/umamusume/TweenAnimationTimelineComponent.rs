use widestring::Utf16Str;

use crate::il2cpp::{api::{il2cpp_class_get_type, il2cpp_type_get_object}, hook::umamusume::TweenAnimationTimelineSheetData, symbols::{IList, get_method_addr}, types::*};
use super::TweenAnimationTimelineData;

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut GETTIMELINEDATA_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetTimelineData, GETTIMELINEDATA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn on_LoadAsset(bundle: *mut Il2CppObject, this: *mut Il2CppObject, name: &Utf16Str) {
    let timeline_data = GetTimelineData(this);
    let Some(sheet_data_list) = IList::new(TweenAnimationTimelineData::get_SheetDataList(timeline_data)) else {
        return;
    };
    for (sheet_data) in sheet_data_list.iter() {
        let name = TweenAnimationTimelineSheetData::get_Name(sheet_data);
        let a2u_prefab = TweenAnimationTimelineSheetData::get_A2UPrefab(sheet_data);
        if !a2u_prefab.is_null() {

        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TweenAnimationTimelineComponent);

    unsafe {
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(TweenAnimationTimelineComponent));
        GETTIMELINEDATA_ADDR = get_method_addr(TweenAnimationTimelineComponent, c"GetTimelineData", 0);
    }
}
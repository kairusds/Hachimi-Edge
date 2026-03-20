use fnv::FnvHashSet;
use widestring::Utf16Str;

use crate::{il2cpp::{api::{il2cpp_class_get_type, il2cpp_type_get_object}, hook::{Plugins::AnimateToUnity::AnRoot, UnityEngine_CoreModule::{GameObject, Object}}, symbols::{IList, get_method_addr}, types::*, ext::Il2CppStringExt}, core::{hachimi::AssetInfo, Hachimi}};
use super::{TweenAnimationTimelineData, TweenAnimationTimelineSheetData};

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut GETTIMELINEDATA_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetTimelineData, GETTIMELINEDATA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn on_LoadAsset(_bundle: *mut Il2CppObject, this: *mut Il2CppObject, _name: &Utf16Str) {
    let timeline_data = GetTimelineData(this);
    let Some(sheet_data_list) = IList::new(TweenAnimationTimelineData::get_SheetDataList(timeline_data)) else {
        return;
    };

    let localized_data = Hachimi::instance().localized_data.load();
    let mut patched_prefabs = FnvHashSet::default();

    for sheet_data in sheet_data_list.iter() {
        let a2u_prefab = TweenAnimationTimelineSheetData::get_A2UPrefab(sheet_data);

        if a2u_prefab.is_null() || !patched_prefabs.insert(a2u_prefab) {
            continue;
        }

        let root = GameObject::GetComponentInChildren(a2u_prefab, AnRoot::type_object(), false);
        if root.is_null() {
            continue;
        }

        let prefab_name = unsafe { Object::get_name(a2u_prefab).as_ref() }
            .map(|s| s.as_utf16str().to_string())
            .unwrap_or_default();

        let mut parts = prefab_name.split('_');

        if parts.next() == Some("pf") && parts.next() == Some("fl") {
            if let Some(category) = parts.next() {
                let base_path = format!("uianimation/flash/{}/prefab/{}", category, prefab_name);

                let asset_info: AssetInfo<AnRoot::AnRootData> = localized_data.load_asset_info(&base_path);
                AnRoot::patch_asset(root, asset_info.data.as_ref());
            }
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
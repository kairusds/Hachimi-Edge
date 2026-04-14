use std::ptr::null_mut;

use fnv::FnvHashMap;
use serde::Deserialize;
use widestring::Utf16Str;

use crate::{
    core::{ext::Utf16StringExt, hachimi::AssetInfo, Hachimi},
    il2cpp::{
        ext::StringExt,
        hook::UnityEngine_AssetBundleModule::AssetBundle::{self, ASSET_PATH_PREFIX},
        symbols::{
            get_field_from_name, get_field_object_value, get_field_value, set_field_object_value,
            IList,
        },
        types::*,
    },
};

#[derive(Deserialize)]
pub struct StoryParamChangeEffectScriptableObjectData {
    #[serde(default)]
    block_data: FnvHashMap<i32, StoryParamChangeEffectBlockDataData>,
}

#[derive(Deserialize)]
struct StoryParamChangeEffectBlockDataData {
    #[serde(default)]
    dataset: FnvHashMap<i32, StoryParamChangeEffectDataSetData>,
}

#[derive(Deserialize)]
struct StoryParamChangeEffectDataSetData {
    #[serde(default)]
    info: FnvHashMap<i32, StoryParamChangeEffectInfoData>,
}

#[derive(Deserialize)]
struct StoryParamChangeEffectInfoData {
    text: Option<String>,
    anim_text: Option<String>,
}

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut PARAM_CHANGE_EFFECT_BLOCK_LIST_FIELD: *mut FieldInfo = null_mut();
pub fn get__paramChangeEffectBlockList(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PARAM_CHANGE_EFFECT_BLOCK_LIST_FIELD })
}

// StoryParamChangeEffectBlockData
static mut BLOCK_INDEX_FIELD: *mut FieldInfo = null_mut();
pub fn get__blockIndex(this: *mut Il2CppObject) -> i32 {
    get_field_value(this, unsafe { BLOCK_INDEX_FIELD })
}
static mut PARAM_CHANGE_EFFECT_DATA_SET_LIST_FIELD: *mut FieldInfo = null_mut();
pub fn get__paramChangeEffectDataSetList(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PARAM_CHANGE_EFFECT_DATA_SET_LIST_FIELD })
}

// StoryParamChangeEffectDataSet
static mut PARAM_CHANGE_INFO_LIST_FIELD: *mut FieldInfo = null_mut();
pub fn get__paramChangeInfoList(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PARAM_CHANGE_INFO_LIST_FIELD })
}

// StoryParamChangeEffectInfo
static mut MESSAGE_TEXT_FIELD: *mut FieldInfo = null_mut();
pub fn set__messageText(this: *mut Il2CppObject, value: *mut Il2CppString) {
    set_field_object_value(this, unsafe { MESSAGE_TEXT_FIELD }, value);
}
static mut ANIMATION_TEXT_FIELD: *mut FieldInfo = null_mut();
pub fn set__animationText(this: *mut Il2CppObject, value: *mut Il2CppString) {
    set_field_object_value(this, unsafe { ANIMATION_TEXT_FIELD }, value);
}

// name: assets/_gallopresources/bundle/resources/story/data/xx/yyyy/ast_param_change_effect_xxyyyyzzz.asset
pub fn on_LoadAsset(bundle: *mut Il2CppObject, this: *mut Il2CppObject, name: &Utf16Str) {
    let base_path = name[ASSET_PATH_PREFIX.len()..].path_basename();
    debug!("Looking for ast_param_change_effect in {}", base_path);

    let localized_data = Hachimi::instance().localized_data.load();
    let asset_info: AssetInfo<StoryParamChangeEffectScriptableObjectData> =
        localized_data.load_asset_info(&base_path.to_string());
    if !AssetBundle::check_asset_bundle_name(bundle, asset_info.metadata_ref()) {
        return;
    }
    patch_asset(this, asset_info.data.as_ref());
}

pub fn patch_asset(
    this: *mut Il2CppObject,
    data_opt: Option<&StoryParamChangeEffectScriptableObjectData>,
) {
    let Some(data) = data_opt else {
        return;
    };
    if data.block_data.is_empty() {
        return;
    }
    let blocks = get__paramChangeEffectBlockList(this);
    let Some(block_list) = IList::new(blocks) else {
        return;
    };
    for block_data_obj in block_list.iter() {
        let block_idx = get__blockIndex(block_data_obj);
        let Some(block_data_dict) = data.block_data.get(&block_idx) else {
            warn!("Block {} not found", block_idx);
            continue;
        };
        if block_data_dict.dataset.is_empty() {
            continue;
        }

        let datasets = get__paramChangeEffectDataSetList(block_data_obj);
        let Some(dataset_list) = IList::new(datasets) else {
            continue;
        };

        for (i, dataset_dict) in block_data_dict.dataset.iter() {
            if dataset_dict.info.is_empty() {
                continue;
            }
            let Some(dataset_data) = dataset_list.get(*i) else {
                warn!("Dataset {} not found", i);
                continue;
            };

            let infos = get__paramChangeInfoList(dataset_data);
            let Some(info_list) = IList::new(infos) else {
                continue;
            };

            for (j, info_dict) in dataset_dict.info.iter() {
                let Some(info_data) = info_list.get(*j) else {
                    warn!("Info {} not found", j);
                    continue;
                };
                if let Some(msg_text) = &info_dict.text {
                    set__messageText(info_data, msg_text.to_il2cpp_string());
                }
                if let Some(anim_text) = &info_dict.anim_text {
                    set__animationText(info_data, anim_text.to_il2cpp_string());
                }
            }
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    // Put everything together since it's very basic.
    get_class_or_return!(umamusume, "Gallop", StoryParamChangeEffectScriptableObject);
    get_class_or_return!(umamusume, "Gallop", StoryParamChangeEffectBlockData);
    get_class_or_return!(umamusume, "Gallop", StoryParamChangeEffectDataSet);
    get_class_or_return!(umamusume, "Gallop", StoryParamChangeEffectInfo);

    unsafe {
        CLASS = StoryParamChangeEffectScriptableObject;
        PARAM_CHANGE_EFFECT_BLOCK_LIST_FIELD = get_field_from_name(
            StoryParamChangeEffectScriptableObject,
            c"_paramChangeEffectBlockList",
        );
        BLOCK_INDEX_FIELD = get_field_from_name(StoryParamChangeEffectBlockData, c"_blockIndex");
        PARAM_CHANGE_EFFECT_DATA_SET_LIST_FIELD = get_field_from_name(
            StoryParamChangeEffectBlockData,
            c"_paramChangeEffectDataSetList",
        );
        PARAM_CHANGE_INFO_LIST_FIELD =
            get_field_from_name(StoryParamChangeEffectDataSet, c"_paramChangeInfoList");

        MESSAGE_TEXT_FIELD = get_field_from_name(StoryParamChangeEffectInfo, c"_messageText");
        ANIMATION_TEXT_FIELD = get_field_from_name(StoryParamChangeEffectInfo, c"_animationText");
    }
}

use crate::{
    core::{Hachimi, game::Region, utils::str_visual_len},
    il2cpp::{ext::{Il2CppStringExt, StringExt}, hook::{UnityEngine_CoreModule::{Component, GameObject, Object, RectTransform, UnityAction}, UnityEngine_UI::{EventSystem, Text}, umamusume::{GallopUtil::without_text_wrap, TextCommon}}, sql::TextDataQuery, symbols::{create_delegate, get_field_from_name, get_field_object_value, get_method_addr}, types::*}
};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use fnv::FnvHashMap;
use super::{ButtonCommon, DialogCommon, DialogManager, MasterDataUtil};

static SKILL_TEXT_CACHE: Lazy<Mutex<FnvHashMap<i32, (String, String)>>> = Lazy::new(|| Mutex::default());

// SkillListItem
static mut NAMETEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get__nameText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAMETEXT_FIELD })
}
static mut DESCTEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get__descText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { DESCTEXT_FIELD })
}

static mut _BGBUTTON_FIELD: *mut FieldInfo = 0 as _;
pub fn get__bgButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { _BGBUTTON_FIELD })
}

static mut INFO_FIELD: *mut FieldInfo = 0 as _;
pub fn get_info(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { INFO_FIELD })
}

static mut LEVELTEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get_levelText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { LEVELTEXT_FIELD })
}

static mut NEED_SKILL_POINT_ROOT_FIELD: *mut FieldInfo = 0 as _;
pub fn get_needSkillPointRoot(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NEED_SKILL_POINT_ROOT_FIELD })
}

static mut set_skill_name_text_addr: usize = 0;
impl_addr_wrapper_fn!(set_skill_name_text, set_skill_name_text_addr, (), this: *mut Il2CppObject);

// PartsSingleModeSkillListItem.Info
static mut get_IsDrawDesc_addr: usize = 0;
impl_addr_wrapper_fn!(get_IsDrawDesc, get_IsDrawDesc_addr, bool, this: *mut Il2CppObject);
static mut get_IsDrawNeedSkillPoint_addr: usize = 0;
impl_addr_wrapper_fn!(get_IsDrawNeedSkillPoint, get_IsDrawNeedSkillPoint_addr, bool, this: *mut Il2CppObject);
static mut get_Id_addr: usize = 0;
impl_addr_wrapper_fn!(get_Id, get_Id_addr, i32, this: *mut Il2CppObject);

fn UpdateItemCommon(this: *mut Il2CppObject, _skill_info: *mut Il2CppObject, orig_fn_cb: impl FnOnce()) {
    //! Must be called first to init the hierarchy correctly.
    without_text_wrap(orig_fn_cb);

    let name = get__nameText(this);
    let desc = get__descText(this);

    if !name.is_null() {
        // Adjust layout width for right-side elements, because they sometimes are full width. WHY.
        let name_transform = Component::get_transform(name);
        let mut right_offset:f32 = 0.0;

        let skill_lvl = get_levelText(this); // Component
        if TextCommon::get_IsActiveInHierarchy(skill_lvl) {
            let lvl_transform = Component::get_transform(skill_lvl);
            right_offset -= RectTransform::get_offsetMax(lvl_transform).x.abs() + Text::get_preferredWidth(skill_lvl);
        }

        let skill_pts = get_needSkillPointRoot(this); // GameObject
        if GameObject::get_activeSelf(skill_pts) {
            let pts_transform = GameObject::get_transform(skill_pts);
            right_offset -= RectTransform::get_rect(pts_transform).width;
        }

        // Set new size if needed.
        if right_offset < 0.0 {
            let mut offset_max = RectTransform::get_offsetMax(name_transform);
            // Remember I said sometimes? Try to detect when.
            if offset_max.x > right_offset {
                offset_max.x = right_offset;
                RectTransform::set_offsetMax(name_transform, offset_max);
            }
        }

        Text::set_best_fit_downscale(name);
    }
    if !desc.is_null() {
        Text::set_best_fit_downscale(desc);
    }
}

type UpdateItemJpFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, adjuster_data: *mut Il2CppObject, resource_hash: i32);
extern "C" fn UpdateItemJp(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, adjuster_data: *mut Il2CppObject, resource_hash: i32) {
    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemJp, UpdateItemJpFn)(this, skill_info, is_plate_effect_enable, adjuster_data, resource_hash);
    });
}

type UpdateItemOtherFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool);
extern "C" fn UpdateItemOther(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool) {
    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemOther, UpdateItemOtherFn)(this, skill_info, is_plate_effect_enable);
    });
}

fn get_skill_text(skill_id: i32) -> (String, String) {
    let to_s = |opt_ptr: Option<*mut Il2CppString>| unsafe {
        opt_ptr.and_then(|p| p.as_ref()).map(|s| s.as_utf16str().to_string())
    };

    let current_name = to_s(TextDataQuery::get_skill_name(skill_id)).unwrap_or_else(|| to_s(Some(MasterDataUtil::GetSkillName(skill_id))).unwrap());
    let current_desc = to_s(TextDataQuery::get_skill_desc(skill_id)).unwrap_or_else(|| to_s(
        Some(Hachimi::instance().skill_info.load().get_desc(skill_id).to_il2cpp_string())
    ).unwrap());

    let mut cache = SKILL_TEXT_CACHE.lock().unwrap();

    if let Some((cached_name, cached_desc)) = cache.get(&skill_id) {
        if cached_name == &current_name && cached_desc == &current_desc {
            return (cached_name.clone(), cached_desc.clone());
        }
    }

    cache.insert(skill_id, (current_name.clone(), current_desc.clone()));
    (current_name, current_desc)
}

type SetupOnClickSkillButtonFn = extern "C" fn(this: *mut Il2CppObject, info: *mut Il2CppObject);
extern "C" fn SetupOnClickSkillButton(this: *mut Il2CppObject, info: *mut Il2CppObject) {
    if !Hachimi::instance().config.load().skill_info_dialog {
        get_orig_fn!(SetupOnClickSkillButton, SetupOnClickSkillButtonFn)(this, info);
        return;
    }
    let skill_id = get_Id(info);
    let button = get__bgButton(this);
    let button_obj = Component::get_gameObject(button);
    Object::set_name(button_obj, format!("HachimiSkill_{}", skill_id).to_il2cpp_string());
    get_skill_text(skill_id);

    let delegate = create_delegate(unsafe { UnityAction::UNITYACTION_CLASS }, 0, || {
        let current_ev = EventSystem::get_current();
        let clicked_obj = EventSystem::get_currentSelectedGameObject(current_ev);
        let object_name = Object::get_name(clicked_obj);
        let name_str = unsafe { (*object_name).as_utf16str() }.to_string();

        if name_str.starts_with("HachimiSkill_") {
            let id_str = &name_str["HachimiSkill_".len()..];
            if let Ok(id) = id_str.parse::<i32>() {
                if let Some(data) = SKILL_TEXT_CACHE.lock().unwrap().get(&id) {
                    let (name, desc) = data;
                    let typ = if str_visual_len(desc.as_str()) <= 250 {
                        DialogCommon::FormType::SMALL_ONE_BUTTON
                    } else if str_visual_len(desc.as_str()) <= 490 {
                        DialogCommon::FormType::MIDDLE_ONE_BUTTON
                    } else {
                        DialogCommon::FormType::BIG_ONE_BUTTON
                    };
                    DialogManager::single_button_message(name, &desc.replace("\\n", "\n"), typ);
                }
            }
        }
    });
    ButtonCommon::SetOnClick(button, delegate.unwrap());
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsSingleModeSkillListItem);
    find_nested_class_or_return!(PartsSingleModeSkillListItem, Info);

    if Hachimi::instance().game.region == Region::Japan {
        let UpdateItem_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 4);
        new_hook!(UpdateItem_addr, UpdateItemJp);
    }
    else {
        let UpdateItem_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 2);
        new_hook!(UpdateItem_addr, UpdateItemOther);
    }

    let SetupOnClickSkillButton_addr = get_method_addr(PartsSingleModeSkillListItem, c"SetupOnClickSkillButton", 1);
    new_hook!(SetupOnClickSkillButton_addr, SetupOnClickSkillButton);

    unsafe {
        // PartsSingleModeSkillListItem
        NAMETEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_nameText");
        DESCTEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_descText");
        LEVELTEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_levelText");
        _BGBUTTON_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_bgButton");
        NEED_SKILL_POINT_ROOT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_needSkillPointRoot");
        INFO_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_info");
        set_skill_name_text_addr = get_method_addr(PartsSingleModeSkillListItem, c"SetSkillNameText", 0);

        // PartsSingleModeSkillListItem.Info
        get_IsDrawDesc_addr = get_method_addr(Info, c"get_IsDrawDesc", 0);
        get_IsDrawNeedSkillPoint_addr = get_method_addr(Info, c"get_IsDrawNeedSkillPoint", 0);
        get_Id_addr = get_method_addr(Info, c"get_Id", 0);
    }
}

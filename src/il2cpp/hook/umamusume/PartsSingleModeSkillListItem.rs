use crate::{
    core::{Hachimi, game::Region, utils::str_visual_len},
    il2cpp::{api::il2cpp_class_from_il2cpp_type, ext::{Il2CppStringExt, StringExt}, hook::{UnityEngine_CoreModule::{Component, GameObject, RectTransform}, UnityEngine_UI::Text, umamusume::{GallopUtil::without_text_wrap, TextCommon}}, sql::TextDataQuery, symbols::{create_delegate, get_field_from_name, get_field_object_value, get_method_addr}, types::*}
};
use super::{DialogCommon, DialogManager, MasterDataUtil};

static mut _ONCLICKBUTTON_FIELD: *mut FieldInfo = 0 as _;
static mut ACTION_INT_CLASS: *mut Il2CppClass = 0 as _;

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

type UpdateItemJpFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, adjuster_data: *mut Il2CppObject, resource_hash: i32, on_click_button: *mut Il2CppObject);
extern "C" fn UpdateItemJp(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, adjuster_data: *mut Il2CppObject, resource_hash: i32, on_click_button: *mut Il2CppObject) {
    let effective_on_click = if Hachimi::instance().config.load().skill_info_dialog && !skill_info.is_null() {
        let on_click_fn: fn() = unsafe { std::mem::transmute(on_click_skill_button as *const ()) };
        create_delegate(unsafe { ACTION_INT_CLASS }, 1, on_click_fn).unwrap() as *mut Il2CppObject
    } else {
        on_click_button
    };

    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemJp, UpdateItemJpFn)(this, skill_info, is_plate_effect_enable, adjuster_data, resource_hash, effective_on_click);
    });
}

// Action<int>
extern "C" fn on_click_skill_button(_ptr: usize, skill_id: i32) {
    info!("on_click_skill_button skill_id {}", skill_id);
    let to_s = |opt_ptr: Option<*mut Il2CppString>| unsafe {
        opt_ptr.and_then(|p| p.as_ref()).map(|s| s.as_utf16str().to_string())
    };

    let skill_name = to_s(TextDataQuery::get_skill_name(skill_id)).unwrap_or_else(|| to_s(Some(MasterDataUtil::GetSkillName(skill_id))).unwrap());
    let skill_desc = to_s(TextDataQuery::get_skill_desc(skill_id)).unwrap_or_else(|| to_s(
        Some(Hachimi::instance().skill_info.load().get_desc(skill_id).to_il2cpp_string())
    ).unwrap());

    let typ = if str_visual_len(skill_desc.as_str()) <= 250 {
        DialogCommon::FormType::SMALL_ONE_BUTTON
    } else if str_visual_len(skill_desc.as_str()) <= 490 {
        DialogCommon::FormType::MIDDLE_ONE_BUTTON
    } else {
        DialogCommon::FormType::BIG_ONE_BUTTON
    };
    DialogManager::single_button_message(&skill_name, &skill_desc.replace("\\n", "\n"), typ);
}

type UpdateItemOtherFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool);
extern "C" fn UpdateItemOther(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool) {
    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemOther, UpdateItemOtherFn)(this, skill_info, is_plate_effect_enable);
    });
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsSingleModeSkillListItem);
    find_nested_class_or_return!(PartsSingleModeSkillListItem, Info);

    if Hachimi::instance().game.region == Region::Japan {
        let UpdateItem_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 5);
        new_hook!(UpdateItem_addr, UpdateItemJp);
    }
    else {
        let UpdateItem_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 2);
        new_hook!(UpdateItem_addr, UpdateItemOther);
    }

    unsafe {
        if Hachimi::instance().game.region != Region::Global {
            _ONCLICKBUTTON_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_onClickButton");
            ACTION_INT_CLASS = il2cpp_class_from_il2cpp_type((*_ONCLICKBUTTON_FIELD).type_);
        }

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

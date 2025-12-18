use crate::{
    core::{Hachimi, game::Region, utils::mul_int},
    il2cpp::{hook::{UnityEngine_UI::Text}, sql::{self, TextDataQuery}, symbols::{get_field_from_name, get_field_object_value, get_method_addr}, types::*}
};

// SkillListItem
static mut NAMETEXT_FIELD: *mut FieldInfo = 0 as _;
fn get__nameText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAMETEXT_FIELD })
}
static mut DESCTEXT_FIELD: *mut FieldInfo = 0 as _;
fn get__descText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { DESCTEXT_FIELD })
}

// SkillInfo
static mut get_IsDrawDesc_addr: usize = 0;
impl_addr_wrapper_fn!(get_IsDrawDesc, get_IsDrawDesc_addr, bool, this: *mut Il2CppObject);
static mut get_IsDrawNeedSkillPoint_addr: usize = 0;
impl_addr_wrapper_fn!(get_IsDrawNeedSkillPoint, get_IsDrawNeedSkillPoint_addr, bool, this: *mut Il2CppObject);

fn UpdateItemCommon(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, orig_fn_cb: impl FnOnce()) {
    let skill_cfg = &Hachimi::instance().localized_data.load().config.skill_formatting;
    let mut txt_cfg = sql::SkillTextFormatting::default();

    let name = get__nameText(this);
    let desc = get__descText(this);

    // Name should always exist, but let's be sure.
    if !name.is_null() {
        let mut name_len = skill_cfg.name_length;
        let mut name_lines = 1;

        // Uma info
        if !get_IsDrawDesc(skill_info) {
            name_len = mul_int(name_len, skill_cfg.name_short_mult);
            name_lines = skill_cfg.name_short_lines;
        }
        // todo: When lvl display!?
        // if get_IsDrawUniqSkillInfo(skill_info) || get_Level(skill_info) > 1 {
        //     name_len = mul_int(name_len, skill_cfg.name_lvl_mult);
        // }
        if get_IsDrawNeedSkillPoint(skill_info) {
            name_len = mul_int(name_len, skill_cfg.name_sp_mult);
        }

        txt_cfg.name = Some(sql::TextFormatting {
            line_len: name_len,
            line_count: name_lines,
            font_size: Text::get_fontSize(name)
        });
    }

    if get_IsDrawDesc(skill_info) && !desc.is_null() {
        let mut desc_len = skill_cfg.desc_length;
        // todo: When conditions button!?
        // if get_IsDisplayUpgradeSkill(skill_info) {
        //     desc_len = mul_int(desc_len, skill_cfg.desc_btn_mult);
        // }

        txt_cfg.desc = Some(sql::TextFormatting {
            line_len: desc_len,
            line_count: 4,
            font_size: Text::get_fontSize(desc)
        });
    }

    TextDataQuery::with_skill_query(&txt_cfg, orig_fn_cb);

    if txt_cfg.is_localized {
        if !name.is_null() {
            Text::set_horizontalOverflow(name, 1);
            if txt_cfg.name.map(|opts| opts.line_count).unwrap_or(1) > 1 {
                Text::set_verticalOverflow(name, 1);
            }
        }
        if !desc.is_null() {
            Text::set_horizontalOverflow(desc, 1);
        }
    }
}

type UpdateItemJpFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, resource_hash: i32);
extern "C" fn UpdateItemJp(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, resource_hash: i32) {
    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemJp, UpdateItemJpFn)(this, skill_info, is_plate_effect_enable, resource_hash);
    });
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
        let UpdateItem_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 3);
        new_hook!(UpdateItem_addr, UpdateItemJp);
    }
    else {
        let UpdateItem_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 2);
        new_hook!(UpdateItem_addr, UpdateItemOther);
    }

    unsafe {
        NAMETEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_nameText");
        DESCTEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_descText");

        // SkillInfo
        get_IsDrawDesc_addr = get_method_addr(Info, c"get_IsDrawDesc", 0);
        get_IsDrawNeedSkillPoint_addr = get_method_addr(Info, c"get_IsDrawNeedSkillPoint", 0);
    }
}

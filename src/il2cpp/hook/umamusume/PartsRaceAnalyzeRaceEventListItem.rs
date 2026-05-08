use crate::il2cpp::{
    hook::umamusume::PartsSingleModeSkillListItem,
    symbols::{get_field_from_name, get_field_object_value, get_method_addr},
    types::*,
};

static mut SKILL_ITEM_FIELD: *mut FieldInfo = 0 as _;
pub fn get_skill_item(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { SKILL_ITEM_FIELD })
}

type SetupFn = extern "C" fn(this: *mut Il2CppObject, list_item_model: *mut Il2CppObject);
fn Setup(this: *mut Il2CppObject, list_item_model: *mut Il2CppObject) {
    get_orig_fn!(Setup, SetupFn)(this, list_item_model);
    let skill_item = get_skill_item(this);
    if !skill_item.is_null() && !PartsSingleModeSkillListItem::get_info(skill_item).is_null() {
        PartsSingleModeSkillListItem::set_skill_name_text(skill_item);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsRaceAnalyzeRaceEventListItem);

    let setup_addr = get_method_addr(PartsRaceAnalyzeRaceEventListItem, c"Setup", 1);
    new_hook!(setup_addr, Setup);

    unsafe {
        SKILL_ITEM_FIELD = get_field_from_name(PartsRaceAnalyzeRaceEventListItem, c"_skillItem");
    }
}

use crate::il2cpp::{
    hook::UnityEngine_UI::Text,
    symbols::{get_field_from_name, get_method_addr},
    types::*,
    utils,
};

def_field_object_accessors!(get get_titleNameText, TITLENAME_TEXT_FIELD, Il2CppObject);
def_field_object_accessors!(get get_textUniqueBonusName, UNIQUEBONUS_NAME_FIELD, Il2CppObject);

const MAX_WIDTH: f32 = 575.0;
const MAX_HEIGHT: f32 = 55.0;

type SetupFn = extern "C" fn(this: *mut Il2CppObject, workSupportCard: *mut Il2CppObject, buttonAction: Il2CppDelegate, hash: i32, enableObtain: bool);
extern "C" fn Setup(this: *mut Il2CppObject, workSupportCard: *mut Il2CppObject, buttonAction: Il2CppDelegate, hash: i32, enableObtain: bool) {
    get_orig_fn!(Setup, SetupFn)(this, workSupportCard, buttonAction, hash, enableObtain);

    let title = get_titleNameText(this);
    if !title.is_null() {
        Text::set_best_fit_downscale(title);
        utils::adjust_transform_size(title, MAX_WIDTH, MAX_HEIGHT);
    }
    let title_as_bonus = get_textUniqueBonusName(this);
    if !title_as_bonus.is_null() {
        Text::set_best_fit_downscale(title_as_bonus);
        utils::adjust_transform_size(title_as_bonus, MAX_WIDTH, MAX_HEIGHT);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsSupportCardImproveDetail);

    let Setup_addr = get_method_addr(PartsSupportCardImproveDetail, c"Setup", 4);
    new_hook!(Setup_addr, Setup);

    unsafe {
        TITLENAME_TEXT_FIELD = get_field_from_name(PartsSupportCardImproveDetail, c"_titleNameText");
        UNIQUEBONUS_NAME_FIELD = get_field_from_name(PartsSupportCardImproveDetail, c"_textUniqueBonusName");
    }
}

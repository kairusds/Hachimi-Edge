use crate::il2cpp::{
    hook::UnityEngine_UI::Text,
    symbols::{get_field_from_name, get_method_addr},
    types::*,
};

def_field_object_accessors!(get get_charaSubTitleText, CHARA_SUBTITLE_TEXT_FIELD, Il2CppObject);
def_field_object_accessors!(get get_charaNameText, CHARA_NAME_TEXT_FIELD, Il2CppObject);

type PlayFadeInFn = extern "C" fn(this: *mut Il2CppObject, onComplete: *mut Il2CppDelegate);
extern "C" fn PlayFadeIn(this: *mut Il2CppObject, onComplete: *mut Il2CppDelegate) {
    let subtitle = get_charaSubTitleText(this);
    let name = get_charaNameText(this);
    if !subtitle.is_null() {
        Text::set_best_fit_downscale(subtitle);
    }
    if !name.is_null() {
        Text::set_best_fit_downscale(name);
    }

    get_orig_fn!(PlayFadeIn, PlayFadeInFn)(this, onComplete);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsNamePlateBase);

    let PlayFadeIn_addr = get_method_addr(PartsNamePlateBase, c"PlayFadeIn", 1);
    new_hook!(PlayFadeIn_addr, PlayFadeIn);

    unsafe {
        CHARA_SUBTITLE_TEXT_FIELD = get_field_from_name(PartsNamePlateBase, c"_charaSubTitleText");
        CHARA_NAME_TEXT_FIELD = get_field_from_name(PartsNamePlateBase, c"_charaNameText");
    }
}

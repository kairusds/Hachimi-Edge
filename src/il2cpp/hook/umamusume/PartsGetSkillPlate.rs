use crate::il2cpp::{
    hook::{UnityEngine_CoreModule::Behaviour, UnityEngine_UI::Text},
    symbols::{get_field_from_name, get_field_object_value, get_method_addr},
    types::*,
    utils,
};

static mut NAME_CONTENTS_SIZE_FITTER_FIELD: *mut FieldInfo = 0 as _;
pub fn get_nameContentsSizeFitter(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAME_CONTENTS_SIZE_FITTER_FIELD })
}

static mut NAME_TEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get_nameText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAME_TEXT_FIELD })
}

// Class handily provides a value. That isn't used…
const SKILL_TEXT_MAX_WIDTH: f32 = 330.0;
// Custom value for this part.
const SKILL_TEXT_MAX_HEIGHT: f32 = 65.0;

type SetUpCharacterLimitBreakSkillFn =
    extern "C" fn(this: *mut Il2CppObject, cardRairtyData: *mut Il2CppObject, nextCardRairtyData: *mut Il2CppObject, atlas: *mut Il2CppObject);
extern "C" fn SetUpCharacterLimitBreakSkill(
    this: *mut Il2CppObject,
    cardRairtyData: *mut Il2CppObject,
    nextCardRairtyData: *mut Il2CppObject,
    atlas: *mut Il2CppObject,
) {
    get_orig_fn!(SetUpCharacterLimitBreakSkill, SetUpCharacterLimitBreakSkillFn)(this, cardRairtyData, nextCardRairtyData, atlas);

    let text = get_nameText(this);
    if !text.is_null() {
        let fitter = get_nameContentsSizeFitter(this);
        if !fitter.is_null() {
            Behaviour::set_enabled(fitter, false);
        }
        utils::adjust_transform_size(text, SKILL_TEXT_MAX_WIDTH, SKILL_TEXT_MAX_HEIGHT);
        Text::set_best_fit_downscale(text);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsGetSkillPlate);
    get_class_or_return!(umamusume, Gallop, PartsGetStatusPlate); // Base class

    let SetUpCharacterLimitBreakSkill_addr = get_method_addr(PartsGetSkillPlate, c"SetUpCharacterLimitBreakSkill", 3);
    new_hook!(SetUpCharacterLimitBreakSkill_addr, SetUpCharacterLimitBreakSkill);

    unsafe {
        NAME_CONTENTS_SIZE_FITTER_FIELD = get_field_from_name(PartsGetSkillPlate, c"_nameContentsSizeFitter");
        NAME_TEXT_FIELD = get_field_from_name(PartsGetStatusPlate, c"_nameText");
    }
}

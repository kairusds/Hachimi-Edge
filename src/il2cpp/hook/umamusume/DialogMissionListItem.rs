use crate::il2cpp::{
    hook::{
        UnityEngine_CoreModule::{
            Component,
            RectTransform::{self, Axis},
        },
        UnityEngine_UI::Text,
    },
    symbols::{get_field_from_name, get_field_object_value, get_method_addr},
    types::*,
};

static mut TITLE_TEXT_FIELD: *mut FieldInfo = 0 as _;
fn get_titleText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { TITLE_TEXT_FIELD })
}
const MISSION_TEXT_HEIGHT: f32 = 87.0;

type SetupFn = extern "C" fn(this: *mut Il2CppObject, parameter: *mut Il2CppObject);
pub fn Setup(this: *mut Il2CppObject, parameter: *mut Il2CppObject) {
    get_orig_fn!(Setup, SetupFn)(this, parameter);
    let text_obj = get_titleText(this);
    let text_transform = Component::get_transform(text_obj);
    RectTransform::SetSizeWithCurrentAnchors(text_transform, Axis::Vertical, MISSION_TEXT_HEIGHT);
    Text::set_best_fit_downscale(text_obj);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DialogMissionListItem);
    let Setup_addr = get_method_addr(DialogMissionListItem, c"Setup", 1);
    new_hook!(Setup_addr, Setup);

    unsafe {
        TITLE_TEXT_FIELD = get_field_from_name(DialogMissionListItem, c"_titleText");
    }
}

use std::ptr::null_mut;

use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        hook::UnityEngine_TextRenderingModule::{TextAnchor, TextGenerator::IgnoreTGFiltersContext},
        symbols::{get_field_from_name, get_field_object_value, get_field_value, get_method_addr, set_field_object_value},
        types::*,
    },
};

static mut TEXT_FIELD: *mut FieldInfo = null_mut();
fn get__text(this: *mut Il2CppObject) -> *mut Il2CppString {
    get_field_object_value(this, unsafe { TEXT_FIELD })
}
fn set__text(this: *mut Il2CppObject, value: *mut Il2CppString) {
    set_field_object_value(this, unsafe { TEXT_FIELD }, value);
}

static mut LINESPACE_FIELD: *mut FieldInfo = null_mut();
pub fn get_lineSpace(this: *mut Il2CppObject) -> f32 {
    get_field_value(this, unsafe { LINESPACE_FIELD })
}

static mut SET_TEXT_FIT_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextFit, SET_TEXT_FIT_ADDR, (), this: *mut Il2CppObject, enable: bool);

static mut SET_TEXT_WRAP_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextWrap, SET_TEXT_WRAP_ADDR, (), this: *mut Il2CppObject, enable: bool);

static mut SET_TEXT_ANCHOR_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextAnchor, SET_TEXT_ANCHOR_ADDR, (), this: *mut Il2CppObject, anchor: TextAnchor);

static mut SET_TEXT_LINESPACE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextLinespace, SET_TEXT_LINESPACE_ADDR, (), this: *mut Il2CppObject, lineSpace: f32);

type _UpdateTextFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn _UpdateText(this: *mut Il2CppObject) {
    let text_ptr = get__text(this);
    if text_ptr.is_null() {
        return get_orig_fn!(_UpdateText, _UpdateTextFn)(this);
    }

    let text = unsafe { (*text_ptr).as_utf16str() };

    // doesn't run through TextGenerator, ignore its filters
    if text.as_slice().contains(&36) { // 36 = dollar sign ($)
        set__text(this, Hachimi::instance().template_parser
            .eval_with_context(&text.to_string(), &mut IgnoreTGFiltersContext())
            .to_il2cpp_string());
    }

    get_orig_fn!(_UpdateText, _UpdateTextFn)(this);
}

pub fn init(Plugins: *const Il2CppImage) {
    get_class_or_return!(Plugins, AnimateToUnity, AnText);

    let _UpdateText_addr = get_method_addr(AnText, c"_UpdateText", 0);

    new_hook!(_UpdateText_addr, _UpdateText);

    unsafe {
        TEXT_FIELD = get_field_from_name(AnText, c"_text");
        LINESPACE_FIELD = get_field_from_name(AnText, c"_lineSpace");

        SET_TEXT_FIT_ADDR = get_method_addr(AnText, c"SetTextFit", 1);
        SET_TEXT_WRAP_ADDR = get_method_addr(AnText, c"SetTextWrap", 1);
        SET_TEXT_ANCHOR_ADDR = get_method_addr(AnText, c"SetTextAnchor", 1);
        SET_TEXT_LINESPACE_ADDR = get_method_addr(AnText, c"SetTextLinespace", 1);
    }
}

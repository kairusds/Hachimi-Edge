use crate::{core::{Hachimi, utils::wrap_fit_text_il2cpp}, il2cpp::{api::{il2cpp_class_get_type, il2cpp_type_get_object}, ext::{Il2CppStringExt, LocalizedDataExt}, hook::UnityEngine_UI::Text, symbols::get_method_addr, types::*}};

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

type AwakeFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Awake(this: *mut Il2CppObject) {
    get_orig_fn!(Awake, AwakeFn)(this);

    let localized_data = Hachimi::instance().localized_data.load();

    let font = localized_data.load_replacement_font();
    if !font.is_null() {
        Text::set_font(this, font);
    }

    if localized_data.config.text_common_allow_overflow {
        Text::set_horizontalOverflow(this, 1);
        Text::set_verticalOverflow(this, 1);
    }
}

// We make the assumption the basic process of these functions is to call
// GallopUtil::LineHeadWrapForSystemText and set_text() the return value.
// The presumed reason those are not called directly is special handling and TextCommon
// object adjustments, which is exactly what we'll do here and take over wrapping.

type SetSystemTextWithLineHeadWrapFn = extern "C" fn(this: *mut Il2CppObject, system_text: *mut CharacterSystemText, maxCharacter: i32);
extern "C" fn SetSystemTextWithLineHeadWrap(this: *mut Il2CppObject, system_text: *mut CharacterSystemText, max_character: i32) {
    let ld = &Hachimi::instance().localized_data.load();
    let systext = unsafe {&*system_text};

    // Only process localized text so as to not possibly fuck up formatting of non-custom text.
    if ld.character_system_text_dict.get(&systext.characterId).and_then(|c| c.get(&systext.voiceId)).is_none() {
        return get_orig_fn!(SetSystemTextWithLineHeadWrap, SetSystemTextWithLineHeadWrapFn)(this, system_text, max_character);
    }

    let cue_sheet = unsafe{(*systext.cueSheet).as_utf16str()}.to_string();
    let cue_type = cue_sheet.split('_').nth(2).unwrap_or_default();
    let font_size = Text::get_fontSize(this);
    debug!("Cue sheet: {}, Font size: {}", cue_type, font_size);

    let max_lines = *ld.config.systext_cue_lines.get(cue_type).unwrap_or_else(||
        ld.config.systext_cue_lines.get("default").unwrap_or(&4)
    );

    // Always fit systext if using wrapper.
    if let Some(wrapped_text) = wrap_fit_text_il2cpp(systext.text, max_character, max_lines, font_size) {
        // Allow wrapper to dictate display.
        Text::set_horizontalOverflow(this, 1);
        Text::set_verticalOverflow(this, 1);
        return Text::set_text(this, wrapped_text);
    }

    get_orig_fn!(SetSystemTextWithLineHeadWrap, SetSystemTextWithLineHeadWrapFn)(this, system_text, max_character);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TextCommon);

    let Awake_addr = get_method_addr(TextCommon, c"Awake", 0);
    new_hook!(Awake_addr, Awake);

    let SetSystemTextWithLineHeadWrap_addr = get_method_addr(TextCommon, c"SetSystemTextWithLineHeadWrap", 2);
    new_hook!(SetSystemTextWithLineHeadWrap_addr, SetSystemTextWithLineHeadWrap);

    unsafe {
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(TextCommon));
    }
}
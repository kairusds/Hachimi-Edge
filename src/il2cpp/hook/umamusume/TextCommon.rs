use crate::{core::{Hachimi, utils::wrap_text_il2cpp}, il2cpp::{api::{il2cpp_class_get_type, il2cpp_type_get_object}, ext::LocalizedDataExt, hook::UnityEngine_UI::Text, symbols::get_method_addr, types::*}};

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

type SetSystemTextWithLineHeadWrapFn = extern "C" fn(this: *mut Il2CppObject, systemText: *mut CharacterSystemText, maxCharacter: i32);
extern "C" fn SetSystemTextWithLineHeadWrap(this: *mut Il2CppObject, systemText: *mut CharacterSystemText, maxCharacter: i32) {
    //? Unsure if a mult is needed vs a static value.
    let config = &Hachimi::instance().localized_data.load().config;
    let base_length = maxCharacter as f32 * config.systext_base_width_multiplier.unwrap_or(1.0);

    // Util func deals with config, etc.
    let Some(wrapped_text) = wrap_text_il2cpp(unsafe {(*systemText).text}, base_length.ceil() as i32) else {
        return get_orig_fn!(SetSystemTextWithLineHeadWrap, SetSystemTextWithLineHeadWrapFn)(this, systemText, maxCharacter)
    };
    set_text(this, wrapped_text);
}

static mut set_text_addr: usize = 0;
impl_addr_wrapper_fn!(set_text, set_text_addr, (), this: *mut Il2CppObject, val: *mut Il2CppString);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TextCommon);

    let Awake_addr = get_method_addr(TextCommon, c"Awake", 0);
    new_hook!(Awake_addr, Awake);

    let SetSystemTextWithLineHeadWrap_addr = get_method_addr(TextCommon, c"SetSystemTextWithLineHeadWrap", 2);
    new_hook!(SetSystemTextWithLineHeadWrap_addr, SetSystemTextWithLineHeadWrap);

    unsafe {
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(TextCommon));
        set_text_addr = get_method_addr(TextCommon, c"set_text", 1);
    }
}
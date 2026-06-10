use std::sync::Mutex;
use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use crate::core::sugoi_client::SugoiClient;
use crate::il2cpp::{ext::{Il2CppStringExt, StringExt}, hook::{UnityEngine_TextRenderingModule::TextAnchor, UnityEngine_CoreModule::Object}, symbols::get_method_addr, types::*};

static mut GET_LINESPACING_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_lineSpacing, GET_LINESPACING_ADDR, f32, this: *mut Il2CppObject);

static mut SET_LINESPACING_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_lineSpacing, SET_LINESPACING_ADDR, (), this: *mut Il2CppObject, value: f32);

static mut GET_FONTSIZE_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_fontSize, GET_FONTSIZE_ADDR, i32, this: *mut Il2CppObject);

static mut SET_FONTSIZE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_fontSize, SET_FONTSIZE_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut SET_FONT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_font, SET_FONT_ADDR, (), this: *mut Il2CppObject, value: *mut Il2CppObject);

static mut SET_HORIZONTALOVERFLOW_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_horizontalOverflow, SET_HORIZONTALOVERFLOW_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut SET_VERTICALOVERFLOW_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_verticalOverflow, SET_VERTICALOVERFLOW_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut GET_TEXT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_text, GET_TEXT_ADDR, *mut Il2CppString, this: *mut Il2CppObject);

static mut SET_TEXT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_text, SET_TEXT_ADDR, (), this: *mut Il2CppObject, value: *mut Il2CppString);

static mut SET_ALIGNMENT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_alignment, SET_ALIGNMENT_ADDR, (), this: *mut Il2CppObject, value: TextAnchor);

static mut GET_PREFERREDHEIGHT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_preferredHeight, GET_PREFERREDHEIGHT_ADDR, f32, this: *mut Il2CppObject);

static mut GET_BEST_FIT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_best_fit, GET_BEST_FIT_ADDR, bool, this: *mut Il2CppObject);

static mut SET_BEST_FIT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_best_fit, SET_BEST_FIT_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut SET_BEST_FIT_MIN_SIZE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_best_fit_min_size, SET_BEST_FIT_MIN_SIZE_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut SET_BEST_FIT_MAX_SIZE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_best_fit_max_size, SET_BEST_FIT_MAX_SIZE_ADDR, (), this: *mut Il2CppObject, value: i32);

static mut GET_PREFERRED_WIDTH_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_preferredWidth, GET_PREFERRED_WIDTH_ADDR, f32, this: *mut Il2CppObject);

pub fn set_best_fit_downscale(this: *mut Il2CppObject) {
    let cur_size = get_fontSize(this);
    set_best_fit_min_size(this, cur_size.min(10));
    set_best_fit_max_size(this, cur_size);
    set_best_fit(this, true);
}

pub static ACTIVE_TEXT_COMPONENTS: Lazy<Mutex<FnvHashMap<usize, String>>> = Lazy::new(|| {
    Mutex::new(FnvHashMap::default())
});

type SetTextFn = extern "C" fn(this: *mut Il2CppObject, value: *mut Il2CppString);
pub extern "C" fn set_text_hook(this: *mut Il2CppObject, value: *mut Il2CppString) {
    if value.is_null() {
        return get_orig_fn!(set_text_hook, SetTextFn)(this, value);
    }

    let config = crate::core::Hachimi::instance().config.load();
    if !config.auto_translate_localize && !config.auto_translate_stories {
        return get_orig_fn!(set_text_hook, SetTextFn)(this, value);
    }

    let orig_str = unsafe { (*value).as_utf16str().to_string() };

    ACTIVE_TEXT_COMPONENTS.lock().unwrap().insert(this as usize, orig_str.clone());

    if let Some(trans) = SugoiClient::instance().get_cached(&orig_str) {
        return get_orig_fn!(set_text_hook, SetTextFn)(this, trans.to_il2cpp_string());
    }

    get_orig_fn!(set_text_hook, SetTextFn)(this, value);
}

pub fn apply_translations(completed: &[(String, String)]) {
    let mut updates_to_apply = Vec::new();
    {
        let mut tracker = ACTIVE_TEXT_COMPONENTS.lock().unwrap();

        tracker.retain(|&ptr, _| Object::op_Implicit(ptr as *mut Il2CppObject));

        for (orig, trans) in completed {
            let unity_string = trans.to_il2cpp_string();

            for (&ptr, saved_orig) in tracker.iter() {
                if saved_orig == orig {
                    updates_to_apply.push((ptr, unity_string));
                }
            }
        }
    }

    for (ptr, unity_string) in updates_to_apply {
        if Object::op_Implicit(ptr as *mut Il2CppObject) {
            get_orig_fn!(set_text_hook, SetTextFn)(ptr as *mut Il2CppObject, unity_string);
        }
    }
}

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.UI", Text);

    let set_text_addr = get_method_addr(Text, c"set_text", 1);
    new_hook!(set_text_addr, set_text_hook);

    unsafe {
        GET_LINESPACING_ADDR = get_method_addr(Text, c"get_lineSpacing", 0);
        SET_LINESPACING_ADDR = get_method_addr(Text, c"set_lineSpacing", 1);
        GET_FONTSIZE_ADDR = get_method_addr(Text, c"get_fontSize", 0);
        SET_FONTSIZE_ADDR = get_method_addr(Text, c"set_fontSize", 1);
        SET_FONT_ADDR = get_method_addr(Text, c"set_font", 1);
        SET_HORIZONTALOVERFLOW_ADDR = get_method_addr(Text, c"set_horizontalOverflow", 1);
        SET_VERTICALOVERFLOW_ADDR = get_method_addr(Text, c"set_verticalOverflow", 1);
        GET_TEXT_ADDR = get_method_addr(Text, c"get_text", 0);
        SET_TEXT_ADDR = get_method_addr(Text, c"set_text", 1);
        SET_ALIGNMENT_ADDR = get_method_addr(Text, c"set_alignment", 1);
        GET_PREFERREDHEIGHT_ADDR = get_method_addr(Text, c"get_preferredHeight", 0);
        GET_PREFERRED_WIDTH_ADDR = get_method_addr(Text, c"get_preferredWidth", 0);
        GET_BEST_FIT_ADDR = get_method_addr(Text, c"get_resizeTextForBestFit", 0);
        SET_BEST_FIT_ADDR = get_method_addr(Text, c"set_resizeTextForBestFit", 1);
        SET_BEST_FIT_MIN_SIZE_ADDR = get_method_addr(Text, c"set_resizeTextMinSize", 1);
        SET_BEST_FIT_MAX_SIZE_ADDR = get_method_addr(Text, c"set_resizeTextMaxSize", 1);
    }
}
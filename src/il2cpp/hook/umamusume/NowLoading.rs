use crate::{core::{utils::truncate_text_il2cpp, Hachimi}, il2cpp::{hook::UnityEngine_UI::Text, symbols::{get_field_from_name, get_field_object_value, get_method_addr}, types::*}};

static mut _COMICTITLE_FIELD: *mut FieldInfo = 0 as _;
fn get__comicTitle(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { _COMICTITLE_FIELD })
}

const COMIC_TITLE_LINE_WIDTH: usize = 23;

type SetupLoadingTipsFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn SetupLoadingTips(this: *mut Il2CppObject) {
    get_orig_fn!(SetupLoadingTips, SetupLoadingTipsFn)(this);

    if Hachimi::instance().localized_data.load().config.now_loading_comic_title_ellipsis {
        let comic_title = get__comicTitle(this);
        if comic_title.is_null() { return; }

        let text = Text::get_text(comic_title);
        if text.is_null() { return; }

        if let Some(new_text) = truncate_text_il2cpp(text, COMIC_TITLE_LINE_WIDTH, true) {
            Text::set_horizontalOverflow(comic_title, 1);
            Text::set_text(comic_title, new_text);
        }
    }
}

type ShowFn = extern "C" fn(this: *mut Il2CppObject, type_: i32, onComplete: *mut Il2CppDelegate, overrideDuration: *mut Il2CppObject, easeType: i32, customInEffect: *mut Il2CppObject, customLoopEffect: *mut Il2CppObject, customOutEffect: *mut Il2CppObject, charaId: i32);
extern "C" fn Show(this: *mut Il2CppObject, mut type_: i32, onComplete: *mut Il2CppDelegate, overrideDuration: *mut Il2CppObject, easeType: i32, customInEffect: *mut Il2CppObject, customLoopEffect: *mut Il2CppObject, customOutEffect: *mut Il2CppObject, charaId: i32) {
    let config = crate::core::Hachimi::instance().config.load();
    #[cfg(target_os = "windows")]
    if type_ == 2 && !config.windows.ui_loading_show_orientation_guide {
        type_ = 0;
    }
    if !config.hide_now_loading {
        get_orig_fn!(Show, ShowFn)(this, type_, onComplete, overrideDuration, easeType, customInEffect, customLoopEffect, customOutEffect, charaId);
    }
    if config.hide_now_loading && !onComplete.is_null() {
        unsafe {
            let invoke: extern "C" fn(*mut Il2CppObject, *const crate::il2cpp::types::MethodInfo) = std::mem::transmute((*onComplete).method_ptr);
            invoke((*onComplete).target, (*onComplete).method);
        }
    }
}

type HideFn = extern "C" fn(this: *mut Il2CppObject, onComplete: *mut Il2CppDelegate, overrideDuration: *mut Il2CppObject, easeType: i32, onUnloadCustomEffectResourcesComplete: *mut Il2CppDelegate);
extern "C" fn Hide(this: *mut Il2CppObject, onComplete: *mut Il2CppDelegate, overrideDuration: *mut Il2CppObject, easeType: i32, onUnloadCustomEffectResourcesComplete: *mut Il2CppDelegate) {
    let config = crate::core::Hachimi::instance().config.load();
    if !config.hide_now_loading {
        get_orig_fn!(Hide, HideFn)(this, onComplete, overrideDuration, easeType, onUnloadCustomEffectResourcesComplete);
    }
    if config.hide_now_loading && !onComplete.is_null() {
        unsafe {
            let invoke: extern "C" fn(*mut Il2CppObject, *const crate::il2cpp::types::MethodInfo) = std::mem::transmute((*onComplete).method_ptr);
            invoke((*onComplete).target, (*onComplete).method);
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, NowLoading);

    let SetupLoadingTips_addr = get_method_addr(NowLoading, c"SetupLoadingTips", 0);
    let show_addr = get_method_addr(NowLoading, c"Show", 8);
    let hide_addr = get_method_addr(NowLoading, c"Hide", 4);

    new_hook!(SetupLoadingTips_addr, SetupLoadingTips);
    new_hook!(show_addr, Show);
    new_hook!(hide_addr, Hide);

    unsafe {
        _COMICTITLE_FIELD = get_field_from_name(NowLoading, c"_comicTitle");
    }
}
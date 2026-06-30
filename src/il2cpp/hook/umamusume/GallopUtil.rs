use std::sync::atomic::{AtomicBool, Ordering};

use crate::{core::{game::Region, utils, Hachimi}, il2cpp::{symbols::get_method_addr, types::*}};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub static NO_WRAP: AtomicBool = AtomicBool::new(false);

pub fn without_text_wrap(callback: impl FnOnce()) {
    NO_WRAP.store(true, Ordering::Relaxed);
    callback();
    NO_WRAP.store(false, Ordering::Relaxed);
}

type LineHeadWrapCommonFnJP = extern "C" fn(
    s: *mut Il2CppString, line_char_count: i32, handling_type: i32, is_match_delegate: *mut Il2CppDelegate
) -> *mut Il2CppString;
extern "C" fn LineHeadWrapCommonJP(
    s: *mut Il2CppString, line_char_count: i32, handling_type: i32, is_match_delegate: *mut Il2CppDelegate
) -> *mut Il2CppString {
    // Don't wrap text if prewrapped or requested.
    if NO_WRAP.load(Ordering::Relaxed) || utils::game_str_has_newline(s) {
        return s;
    }

    if let Some(wrapped) = utils::wrap_text_il2cpp(s, line_char_count) {
        return wrapped;
    }
    get_orig_fn!(LineHeadWrapCommonJP, LineHeadWrapCommonFnJP)(s, line_char_count, handling_type, is_match_delegate)
}

type LineHeadWrapCommonFnGlobal = extern "C" fn(
    s: *mut Il2CppString, line_char_count: i32, is_match_delegate: *mut Il2CppDelegate
) -> *mut Il2CppString;
extern "C" fn LineHeadWrapCommonGlobal(
    s: *mut Il2CppString, line_char_count: i32, is_match_delegate: *mut Il2CppDelegate
) -> *mut Il2CppString {
    if utils::game_str_has_newline(s) {
        // assume prewrapped, let the game handle it
        return get_orig_fn!(LineHeadWrapCommonGlobal, LineHeadWrapCommonFnGlobal)(s, line_char_count, is_match_delegate);
    }

    if let Some(wrapped) = utils::wrap_text_il2cpp(s, line_char_count) {
        return wrapped;
    }
    get_orig_fn!(LineHeadWrapCommonGlobal, LineHeadWrapCommonFnGlobal)(s, line_char_count, is_match_delegate)
}

type LineHeadWrapCommonWithColorTagFn = extern "C" fn(
    str: *mut Il2CppString, line_char_count: i32, is_count_single_char: bool, is_match_delegate: *mut Il2CppDelegate
) -> *mut Il2CppString;
extern "C" fn LineHeadWrapCommonWithColorTag(
    str: *mut Il2CppString, line_char_count: i32, is_count_single_char: bool, is_match_delegate: *mut Il2CppDelegate
) -> *mut Il2CppString {
    if let Some(wrapped) = utils::wrap_text_il2cpp(str, line_char_count) {
        return wrapped;
    }
    get_orig_fn!(LineHeadWrapCommonWithColorTag, LineHeadWrapCommonWithColorTagFn)(
        str, line_char_count, is_count_single_char, is_match_delegate
    )
}

// public static String LineHeadWrap(String text, Int32 lineCharCount) { }
static mut LINEHEADWRAP_ADDR: usize = 0;
impl_addr_wrapper_fn!(LineHeadWrap, LINEHEADWRAP_ADDR, *mut Il2CppString, text: *mut Il2CppString, line_char_count: i32);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, GallopUtil);

    unsafe {
        CLASS = GallopUtil;
        LINEHEADWRAP_ADDR = get_method_addr(GallopUtil, c"LineHeadWrap", 2);
    }

    let region = &Hachimi::instance().game.region;

    match region {
        &Region::Japan => {
            let LineHeadWrapCommon_addr = get_method_addr(GallopUtil, c"LineHeadWrapCommon", 4);
            new_hook!(LineHeadWrapCommon_addr, LineHeadWrapCommonJP);
        }
        _ => {
            let LineHeadWrapCommon_addr = get_method_addr(GallopUtil, c"LineHeadWrapCommon", 3);
            new_hook!(LineHeadWrapCommon_addr, LineHeadWrapCommonGlobal);
        }
    }

    let LineHeadWrapCommonWithColorTag_addr = get_method_addr(GallopUtil, c"LineHeadWrapCommonWithColorTag", 4);
    new_hook!(LineHeadWrapCommonWithColorTag_addr, LineHeadWrapCommonWithColorTag);
}

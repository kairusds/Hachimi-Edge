use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        hook::UnityEngine_TextRenderingModule::TextGenerator::IgnoreTGFiltersContext,
        symbols::get_method_addr,
        types::{Il2CppClass, Il2CppObject, Il2CppDelegate, Il2CppString},
    },
};

fn process_text(text: *mut Il2CppString) -> *mut Il2CppString {
    if !text.is_null() {
        let utf_str = unsafe { (*text).as_utf16str() };
        // doesn't run through TextGenerator, ignore its filters
        // 36 = dollar sign ($)
        if utf_str.as_slice().contains(&36) {
            let clean_text = Hachimi::instance()
                .template_parser
                .eval_with_context(&utf_str.to_string(), &mut IgnoreTGFiltersContext());
            return clean_text.to_il2cpp_string();
        }
    }
    text
}

type SetTextFn = extern "C" fn(this: *mut Il2CppObject, text: *mut Il2CppString);
extern "C" fn SetText(this: *mut Il2CppObject, text: *mut Il2CppString) {
    let text = process_text(text);
    get_orig_fn!(SetText, SetTextFn)(this, text);
}

type PlayFn = extern "C" fn(this: *mut Il2CppObject, text: *mut Il2CppString, callback: *mut Il2CppDelegate);
extern "C" fn Play(this: *mut Il2CppObject, text: *mut Il2CppString, callback: *mut Il2CppDelegate) {
    let text = process_text(text);
    get_orig_fn!(Play, PlayFn)(this, text, callback);
}

pub fn init(PartsCommonHeaderTitle: *mut Il2CppClass) {
    find_nested_class_or_return!(PartsCommonHeaderTitle, TitlePlayer);

    match Hachimi::instance().game.region {
        Region::Japan | Region::Taiwan => {
            let SetText_addr = get_method_addr(TitlePlayer, c"SetText", 1);
            new_hook!(SetText_addr, SetText);
        }
        _ => {
            let Play_addr = get_method_addr(TitlePlayer, c"Play", 2);
            new_hook!(Play_addr, Play);
        }
    }
}

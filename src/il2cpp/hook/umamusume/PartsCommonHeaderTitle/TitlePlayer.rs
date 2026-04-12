use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        hook::UnityEngine_TextRenderingModule::TextGenerator::IgnoreTGFiltersContext,
        symbols::get_method_addr,
        types::{Il2CppClass, Il2CppObject, Il2CppString},
    },
};

type SetTextFn = extern "C" fn(this: *mut Il2CppObject, text: *mut Il2CppString);
extern "C" fn SetText(this: *mut Il2CppObject, text: *mut Il2CppString) {
    let utf_str = unsafe { (*text).as_utf16str() };
    // doesn't run through TextGenerator, ignore its filters
    // 36 = dollar sign ($)
    if !text.is_null() && utf_str.as_slice().contains(&36) {
        let clean_text = Hachimi::instance()
            .template_parser
            .eval_with_context(&utf_str.to_string(), &mut IgnoreTGFiltersContext());
        return get_orig_fn!(SetText, SetTextFn)(this, clean_text.to_il2cpp_string());
    }

    get_orig_fn!(SetText, SetTextFn)(this, text);
}

pub fn init(PartsCommonHeaderTitle: *mut Il2CppClass) {
    find_nested_class_or_return!(PartsCommonHeaderTitle, TitlePlayer);

    let SetText_addr = get_method_addr(TitlePlayer, c"SetText", 1);

    new_hook!(SetText_addr, SetText);
}
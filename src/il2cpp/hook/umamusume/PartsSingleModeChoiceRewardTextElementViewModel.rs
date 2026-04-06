use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        hook::UnityEngine_TextRenderingModule::TextGenerator::IgnoreTGFiltersContext,
        symbols::get_method_addr,
        types::{Il2CppImage, Il2CppObject, Il2CppString},
    },
};

type GetParameterValueTextFn =
    extern "C" fn(this: *mut Il2CppObject, param: i32) -> *mut Il2CppString;
extern "C" fn GetParameterValueText(this: *mut Il2CppObject, param: i32) -> *mut Il2CppString {
    let mut text = get_orig_fn!(GetParameterValueText, GetParameterValueTextFn)(this, param);
    let utf_str = unsafe { (*text).as_utf16str() };
    if utf_str.as_slice().contains(&36) {
        text = Hachimi::instance()
            .template_parser
            .eval_with_context(&utf_str.to_string(), &mut IgnoreTGFiltersContext())
            .to_il2cpp_string();
    }
    text
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(
        umamusume,
        Gallop,
        PartsSingleModeChoiceRewardTextElementViewModel
    );

    let GetParameterValueText_addr = get_method_addr(
        PartsSingleModeChoiceRewardTextElementViewModel,
        c"GetParameterValueText",
        1,
    );
    new_hook!(GetParameterValueText_addr, GetParameterValueText);
}

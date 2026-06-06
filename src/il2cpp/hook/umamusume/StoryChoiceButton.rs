use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        hook::{Plugins::AnimateToUnity::AnText, UnityEngine_TextRenderingModule::TextAnchor},
        symbols::get_method_addr,
        types::*,
    },
};

type SetupFn = extern "C" fn(
    this: *mut Il2CppObject,
    labelObject: *mut Il2CppString,
    textLabel: *mut Il2CppString,
    imageObjectName: *mut Il2CppString,
    anObject: *mut *mut Il2CppObject,
    anText: *mut *mut Il2CppObject,
    imageCommon: *mut *mut Il2CppObject,
    canvasGroup: *mut *mut Il2CppObject,
);
extern "C" fn Setup(
    this: *mut Il2CppObject,
    labelObject: *mut Il2CppString,
    textLabel: *mut Il2CppString,
    imageObjectName: *mut Il2CppString,
    anObject: *mut *mut Il2CppObject,
    anText: *mut *mut Il2CppObject,
    imageCommon: *mut *mut Il2CppObject,
    canvasGroup: *mut *mut Il2CppObject,
) {
    // Called at the start of a story for each of the 6 possible buttons.
    get_orig_fn!(Setup, SetupFn)(this, labelObject, textLabel, imageObjectName, anObject, anText, imageCommon, canvasGroup);

    if anText.is_null() {
        return;
    }
    let an_text = unsafe { *anText };

    AnText::SetTextAnchor(an_text, TextAnchor::MiddleLeft);
    AnText::SetTextWrap(an_text, true);
    // Wrap and fit don't work together by default and I can't find how to set the max height or lines.
    // With nice wrap, tl is unlikely to need more than 2 lines though and RichText can still be used.
    // AnText::SetTextFit(anText, true);

    let config = &Hachimi::instance().localized_data.load().config;
    if let Some(mult) = config.choice_btn_line_spacing_multiplier {
        let line_spacing = AnText::get_lineSpace(an_text);
        AnText::SetTextLinespace(an_text, line_spacing * mult);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, StoryChoiceButton);

    let Setup_addr = get_method_addr(StoryChoiceButton, c"Setup", 7);
    new_hook!(Setup_addr, Setup);
}

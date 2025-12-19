use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        ext::{Il2CppStringExt, StringExt}, hook::UnityEngine_TextRenderingModule::TextGenerator::IgnoreTGFiltersContext, symbols::get_method_addr, types::*
    }
};

fn PlayTypeWriteCommon(mut message: *mut Il2CppString) -> *mut Il2CppString {
    let message_utf16 = unsafe { (*message).as_utf16str() };
    if message_utf16.as_slice().contains(&36) { // 36 = dollar sign ($)
        message = Hachimi::instance().template_parser
            .eval_with_context(&message_utf16.to_string(), &mut IgnoreTGFiltersContext())
            .to_il2cpp_string()
    }
    message
}

type PlayTypeWriteJpFn = extern "C" fn(this: *mut Il2CppObject, message: *mut Il2CppString, skip_add_system_log: bool);
extern "C" fn PlayTypeWriteJp(this: *mut Il2CppObject, mut message: *mut Il2CppString, skip_add_system_log: bool) {
    if !message.is_null() {
        message = PlayTypeWriteCommon(message);
    }
    get_orig_fn!(PlayTypeWriteJp, PlayTypeWriteJpFn)(this, message, skip_add_system_log)
}

type PlayTypeWriteOtherFn = extern "C" fn(this: *mut Il2CppObject, message: *mut Il2CppString);
extern "C" fn PlayTypeWriteOther(this: *mut Il2CppObject, mut message: *mut Il2CppString) {
    if !message.is_null() {
        message = PlayTypeWriteCommon(message);
    }
    get_orig_fn!(PlayTypeWriteOther, PlayTypeWriteOtherFn)(this, message)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TrainingParamChangePlate);

    if Hachimi::instance().game.region == Region::Japan {
        let PlayTypeWrite_addr = get_method_addr(TrainingParamChangePlate, c"PlayTypeWrite", 2);
        new_hook!(PlayTypeWrite_addr, PlayTypeWriteJp);
    }
    else {
        let PlayTypeWrite_addr = get_method_addr(TrainingParamChangePlate, c"PlayTypeWrite", 1);
        new_hook!(PlayTypeWrite_addr, PlayTypeWriteOther);
    }
}
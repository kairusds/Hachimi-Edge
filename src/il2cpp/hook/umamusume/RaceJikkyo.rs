use crate::{
    core::Hachimi,
    il2cpp::{
        ext::StringExt,
        symbols::{get_field_from_name, get_field_value, get_method_addr, set_field_object_value},
        types::*,
    },
};

static mut MESSAGE_ID_FIELD: *mut FieldInfo = std::ptr::null_mut();
static mut MESSAGE_TEXT_FIELD: *mut FieldInfo = std::ptr::null_mut();
static mut COMMENT_ID_FIELD: *mut FieldInfo = std::ptr::null_mut();
static mut COMMENT_TEXT_FIELD: *mut FieldInfo = std::ptr::null_mut();

type CacheFn = extern "C" fn(this: *mut Il2CppObject, tag_list: *mut Il2CppArray);

extern "C" fn CacheMessage(this: *mut Il2CppObject, tag_list: *mut Il2CppArray) {
    let id: i32 = get_field_value(this, unsafe { MESSAGE_ID_FIELD });
    let localized_data = Hachimi::instance().localized_data.load();

    if let Some(text) = localized_data.race_jikkyo_message_dict.get(&id) {
        let text = text.to_il2cpp_string();
        set_field_object_value(this, unsafe { MESSAGE_TEXT_FIELD }, text);
    }

    get_orig_fn!(CacheMessage, CacheFn)(this, tag_list);
}

extern "C" fn CacheComment(this: *mut Il2CppObject, tag_list: *mut Il2CppArray) {
    let id: i32 = get_field_value(this, unsafe { COMMENT_ID_FIELD });
    let localized_data = Hachimi::instance().localized_data.load();

    if let Some(text) = localized_data.race_jikkyo_comment_dict.get(&id) {
        let text = text.to_il2cpp_string();
        set_field_object_value(this, unsafe { COMMENT_TEXT_FIELD }, text);
    }

    get_orig_fn!(CacheComment, CacheFn)(this, tag_list);
}

fn init_message(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, MasterRaceJikkyoMessage);
    find_nested_class_or_return!(MasterRaceJikkyoMessage, RaceJikkyoMessage);

    let id_field = get_field_from_name(RaceJikkyoMessage, c"Id");
    let text_field = get_field_from_name(RaceJikkyoMessage, c"Message");
    if id_field.is_null() || text_field.is_null() {
        return;
    }

    unsafe {
        MESSAGE_ID_FIELD = id_field;
        MESSAGE_TEXT_FIELD = text_field;
    }

    let cache_addr = get_method_addr(RaceJikkyoMessage, c"Cache", 1);
    new_hook!(cache_addr, CacheMessage);
}

fn init_comment(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, MasterRaceJikkyoComment);
    find_nested_class_or_return!(MasterRaceJikkyoComment, RaceJikkyoComment);

    let id_field = get_field_from_name(RaceJikkyoComment, c"Id");
    let text_field = get_field_from_name(RaceJikkyoComment, c"Message");
    if id_field.is_null() || text_field.is_null() {
        return;
    }

    unsafe {
        COMMENT_ID_FIELD = id_field;
        COMMENT_TEXT_FIELD = text_field;
    }

    let cache_addr = get_method_addr(RaceJikkyoComment, c"Cache", 1);
    new_hook!(cache_addr, CacheComment);
}

pub fn init(umamusume: *const Il2CppImage) {
    init_message(umamusume);
    init_comment(umamusume);
}

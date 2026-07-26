use std::{
    ptr::null_mut,
    sync::RwLock
};
use crate::il2cpp::{
    api::{il2cpp_class_get_type, il2cpp_type_get_object},
    ext::StringExt, hook::mscorlib::Enum, symbols::IEnumerable, types::*
};
use once_cell::sync::Lazy;
use fnv::FnvHashMap;

static mut TEXTID_TYPE_OBJECT: *mut Il2CppObject = null_mut();

static TEXTID_NAME_ID_CACHE: Lazy<RwLock<FnvHashMap<String, i32>>> =  Lazy::new(|| RwLock::new(FnvHashMap::default()));

// Mandatory for using get_from_name()
pub fn cache_name_id(name: &str) {
    if TEXTID_NAME_ID_CACHE.read().unwrap().contains_key(name) {
        return;
    }
    let id = from_name(name);
    TEXTID_NAME_ID_CACHE.write().unwrap().insert(name.to_string(), id);
}

// Thread-safe alternative to from_name()
pub fn get_from_name(name: &str) -> Option<i32> {
    TEXTID_NAME_ID_CACHE.read().unwrap().get(name).copied()
}

pub fn get_name(value: i32) -> *const Il2CppString {
    let text_id = Enum::ToObject(unsafe { TEXTID_TYPE_OBJECT }, value);
    Enum::ToString(text_id)
}

// this is named like a constructor to pretend that i32 = TextId
// because that's how it's represented in il2cpp
pub fn from_name(name: &str) -> i32 {
    let text_id = Enum::Parse(unsafe { TEXTID_TYPE_OBJECT }, name.to_il2cpp_string());
    Enum::ToUInt64(text_id) as i32
}

pub fn get_values() -> IEnumerable {
    Enum::GetValues(unsafe { TEXTID_TYPE_OBJECT })
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TextId);

    unsafe {
        TEXTID_TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(TextId));
    }
}
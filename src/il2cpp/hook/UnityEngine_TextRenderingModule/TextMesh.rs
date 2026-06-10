use std::sync::Mutex;
use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use crate::core::sugoi_client::SugoiClient;
use crate::il2cpp::{ext::{Il2CppStringExt, StringExt}, hook::UnityEngine_CoreModule::Object, symbols::get_method_addr, types::*};

pub static ACTIVE_TEXT_MESH_COMPONENTS: Lazy<Mutex<FnvHashMap<usize, String>>> = Lazy::new(|| {
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

    ACTIVE_TEXT_MESH_COMPONENTS.lock().unwrap().insert(this as usize, orig_str.clone());

    if let Some(trans) = SugoiClient::instance().get_cached(&orig_str) {
        return get_orig_fn!(set_text_hook, SetTextFn)(this, trans.to_il2cpp_string());
    }

    get_orig_fn!(set_text_hook, SetTextFn)(this, value);
}

pub fn apply_translations(completed: &[(String, String)]) {
    let mut updates_to_apply = Vec::new();
    {
        let mut tracker = ACTIVE_TEXT_MESH_COMPONENTS.lock().unwrap();

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

pub fn init(UnityEngine_TextRenderingModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_TextRenderingModule, UnityEngine, TextMesh);

    let set_text_addr = get_method_addr(TextMesh, c"set_text", 1);
    new_hook!(set_text_addr, set_text_hook);
}
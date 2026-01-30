use std::{
    collections::HashMap,
    ffi::CStr,
    sync::{atomic::{AtomicU64, Ordering}, Mutex},
};

use jni::{
    objects::{GlobalRef, JClass, JObject, JValue},
    JNIEnv,
};
use once_cell::sync::Lazy;

use crate::android::main::java_vm;

fn log_exception(env: &mut JNIEnv, context: &str) {
    if env.exception_check().unwrap_or(false) {
        log::warn!("dex_bridge: JNI exception during {}", context);
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

#[derive(Clone)]
struct DexEntry {
    class_loader: GlobalRef,
    class_obj: GlobalRef,
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
static DEX_REGISTRY: Lazy<Mutex<HashMap<u64, DexEntry>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn get_activity<'a>(env: &mut JNIEnv<'a>) -> Option<JObject<'a>> {
    let activity_thread_class = env.find_class("android/app/ActivityThread").ok()?;
    let activity_thread = env
        .call_static_method(
            activity_thread_class,
            "currentActivityThread",
            "()Landroid/app/ActivityThread;",
            &[],
        )
        .ok()?
        .l()
        .ok()?;
    let activities = env
        .get_field(activity_thread, "mActivities", "Landroid/util/ArrayMap;")
        .ok()?
        .l()
        .ok()?;
    let activities_map = jni::objects::JMap::from_env(env, &activities).ok()?;
    let (_, activity_record) = activities_map.iter(env).ok()?.next(env).ok()??;
    let activity = env
        .get_field(activity_record, "activity", "Landroid/app/Activity;")
        .ok()?
        .l()
        .ok()?;
    Some(activity)
}

fn load_class_from_dex(env: &mut JNIEnv, dex_bytes: &[u8], class_name: &str) -> Option<(GlobalRef, GlobalRef)> {
    let activity = match get_activity(env) {
        Some(activity) => activity,
        None => {
            log::warn!("dex_bridge: no Activity found");
            return None;
        }
    };
    let class_loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .ok()?
        .l()
        .ok()?;

    let byte_array = env.byte_array_from_slice(dex_bytes).ok()?;
    let byte_buffer = env
        .call_static_method(
            "java/nio/ByteBuffer",
            "wrap",
            "([B)Ljava/nio/ByteBuffer;",
            &[JValue::Object(&JObject::from(byte_array))],
        )
        .ok()?
        .l()
        .ok()?;

    let dex_loader = env
        .new_object(
            "dalvik/system/InMemoryDexClassLoader",
            "(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V",
            &[JValue::Object(&byte_buffer), JValue::Object(&class_loader)],
        )
        .ok()?;

    let class_name = env.new_string(class_name).ok()?;
    let class_obj = env
        .call_method(
            &dex_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name)],
        )
        .ok()
        .and_then(|v| v.l().ok())?;

    let loader_ref = env.new_global_ref(&dex_loader).ok()?;
    let class_ref = env.new_global_ref(class_obj).ok()?;
    Some((loader_ref, class_ref))
}

fn with_env<F: FnOnce(&mut JNIEnv) -> bool>(f: F) -> bool {
    let Some(vm) = java_vm() else { return false; };
    let Ok(mut env) = vm.attach_current_thread() else { return false; };
    f(&mut env)
}

pub fn dex_load(dex_ptr: *const u8, dex_len: usize, class_name: *const std::os::raw::c_char) -> u64 {
    if dex_ptr.is_null() || dex_len == 0 {
        return 0;
    }
    if class_name.is_null() {
        return 0;
    }
    let Ok(class_name) = unsafe { CStr::from_ptr(class_name) }.to_str() else { return 0; };

    let dex_bytes = unsafe { std::slice::from_raw_parts(dex_ptr, dex_len) };

    let mut handle_out = 0;
    let ok = with_env(|env| {
        let Some((loader_ref, class_ref)) = load_class_from_dex(env, dex_bytes, class_name) else {
            log_exception(env, "dex_load");
            return false;
        };
        let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        DEX_REGISTRY.lock().unwrap().insert(
            handle,
            DexEntry { class_loader: loader_ref, class_obj: class_ref },
        );
        handle_out = handle;
        true
    });

    if ok { handle_out } else { 0 }
}

pub fn dex_unload(handle: u64) -> bool {
    DEX_REGISTRY.lock().unwrap().remove(&handle).is_some()
}

pub fn call_static_noargs(handle: u64, method: &CStr, sig: &CStr) -> bool {
    let Ok(method) = method.to_str() else { return false; };
    let Ok(sig) = sig.to_str() else { return false; };
    let entry = DEX_REGISTRY.lock().unwrap().get(&handle).cloned();
    let Some(entry) = entry else { return false; };

    with_env(|env| {
        let Ok(class_obj) = env.new_local_ref(entry.class_obj.as_obj()) else { return false; };
        let class = JClass::from(class_obj);
        match env.call_static_method(class, method, sig, &[]) {
            Ok(_) => true,
            Err(_) => {
                log::warn!("dex_bridge: call_static_noargs failed ({} {})", method, sig);
                log_exception(env, "call_static_noargs");
                false
            }
        }
    })
}

pub fn call_static_string(handle: u64, method: &CStr, sig: &CStr, arg: &CStr) -> bool {
    let Ok(method) = method.to_str() else { return false; };
    let Ok(sig) = sig.to_str() else { return false; };
    let Ok(arg_str) = arg.to_str() else { return false; };
    let entry = DEX_REGISTRY.lock().unwrap().get(&handle).cloned();
    let Some(entry) = entry else { return false; };

    with_env(|env| {
        let Ok(class_obj) = env.new_local_ref(entry.class_obj.as_obj()) else { return false; };
        let class = JClass::from(class_obj);
        let Ok(jarg) = env.new_string(arg_str) else { return false; };
        match env.call_static_method(class, method, sig, &[JValue::Object(&jarg)]) {
            Ok(_) => true,
            Err(_) => {
                log::warn!("dex_bridge: call_static_string failed ({} {})", method, sig);
                log_exception(env, "call_static_string");
                false
            }
        }
    })
}

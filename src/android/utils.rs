use jni::{
    objects::{JValue, JMap, JObject, JString},
    JNIEnv
};
use crate::{
    android::main::java_vm,
    il2cpp::{ext::StringExt, hook::UnityEngine_CoreModule::Application}
};

use std::{path::PathBuf, sync::atomic::{AtomicBool, Ordering}};
use super::game_impl;

pub static BACK_BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);
pub static IS_IME_VISIBLE: AtomicBool = AtomicBool::new(false);

pub fn set_keyboard_visible(visible: bool) {
    let vm = java_vm().expect("JavaVM not initialized");
    let mut env = vm.attach_current_thread().expect("Failed to attach thread");

    let result = (|| -> jni::errors::Result<()> {
        let activity = get_activity(unsafe { env.unsafe_clone() })
            .ok_or(jni::errors::Error::JavaException)?;

        // get InputMethodManager: context.getSystemService(Context.INPUT_METHOD_SERVICE)
        let context_class = env.find_class("android/content/Context")?;
        let imm_service_name = env.get_static_field(context_class, "INPUT_METHOD_SERVICE", "Ljava/lang/String;")?.l()?;

        let imm = env.call_method(
            &activity, 
            "getSystemService", 
            "(Ljava/lang/String;)Ljava/lang/Object;", 
            &[JValue::from(&imm_service_name)]
        )?.l()?;

        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        let decor_view = env.call_method(window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
        let window_token = env.call_method(&decor_view, "getWindowToken", "()Landroid/os/IBinder;", &[])?.l()?;

        if visible {
            // show: imm.showSoftInput(view, flags)
            // SHOW_IMPLICIT (1) or SHOW_FORCED (2)
            let shown = env.call_method(
                &imm, 
                "showSoftInput", 
                "(Landroid/view/View;I)Z", 
                &[JValue::from(&decor_view), JValue::Int(2)]
            )?.z()?;

            if !shown {
                env.call_method(
                    &imm, 
                    "toggleSoftInput", 
                    "(II)V", 
                    &[JValue::Int(2), JValue::Int(1)]
                )?;
            }
            IS_IME_VISIBLE.store(true, Ordering::Release);
        } else {
            // hide: imm.hideSoftInputFromWindow(token, flags)
            env.call_method(
                &imm, 
                "hideSoftInputFromWindow", 
                "(Landroid/os/IBinder;I)Z", 
                &[JValue::from(&window_token), JValue::Int(0)]
            )?;
            IS_IME_VISIBLE.store(false, Ordering::Release);
        }
        Ok(())
    })();

    if let Err(e) = result {
        info!("JNI Keyboard Error: {:?}", e);
    }
}

pub fn check_keyboard_status() -> bool {
    let vm = java_vm().unwrap();
    let mut env = vm.attach_current_thread().unwrap();
    let api_level = get_device_api_level(env.get_native_interface());

    let is_visible = (|| -> jni::errors::Result<bool> {
        let activity = get_activity(unsafe { env.unsafe_clone() }).ok_or(jni::errors::Error::JavaException)?;
        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        let decor_view = env.call_method(window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;

        if api_level >= 30 {
            // Android 11+
            let root_insets = env.call_method(&decor_view, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?.l()?;
            if !root_insets.is_null() {
                let ime_type = 8; // WindowInsets.Type.ime()
                return env.call_method(root_insets, "isVisible", "(I)Z", &[JValue::Int(ime_type)])?.z();
            }
        } 
        
        // fallback for Android < 11: check Rect height difference
        let rect_class = env.find_class("android/graphics/Rect")?;
        let rect_obj = env.new_object(&rect_class, "()V", &[])?;
        env.call_method(&decor_view, "getWindowVisibleDisplayFrame", "(Landroid/graphics/Rect;)V", &[JValue::from(&rect_obj)])?;
        
        let display_height = env.call_method(&decor_view, "getHeight", "()I", &[])?.i()?;
        let visible_bottom = env.get_field(&rect_obj, "bottom", "I")?.i()?;
        
        // if the gap between bottom of screen and bottom of visible area is > 200dp, keyboard is likely up
        let height_diff = display_height - visible_bottom;
        Ok(height_diff > (display_height / 4)) // using 25% of screen as threshold
    })().unwrap_or(false);

    let old = IS_IME_VISIBLE.swap(is_visible, Ordering::AcqRel);
    if old != is_visible {
        info!("Keyboard visibility changed: {}", is_visible);
    }
    is_visible
}

pub fn open_app_or_fallback(package_name: &str, activity_class: &str, fallback_url: &str) {
    let vm = match java_vm() {
        Some(v) => v,
        None => return,
    };

    let mut env = match vm.attach_current_thread() {
        Ok(e) => e,
        Err(_) => return,
    };

    let try_open = |env: &mut JNIEnv| -> jni::errors::Result<()> {
        let activity = get_activity(unsafe { env.unsafe_clone() }).ok_or(jni::errors::Error::JavaException)?;

        let intent_class = env.find_class("android/content/Intent")?;

        let intent_obj = env.new_object(&intent_class, "()V", &[])?;

        let pkg_name_java = env.new_string(package_name)?;
        let cls_name_java = env.new_string(activity_class)?;
        
        let component_class = env.find_class("android/content/ComponentName")?;
        let component_obj = env.new_object(
            &component_class, 
            "(Ljava/lang/String;Ljava/lang/String;)V", 
            &[JValue::from(&pkg_name_java), JValue::from(&cls_name_java)]
        )?;
    
        env.call_method(
            &intent_obj, 
            "setComponent", 
            "(Landroid/content/ComponentName;)Landroid/content/Intent;", 
            &[JValue::from(&component_obj)]
        )?;

        env.call_method(&intent_obj, "setFlags", "(I)Landroid/content/Intent;", &[JValue::Int(0x10000000)])?;

        env.call_method(&activity, "startActivity", "(Landroid/content/Intent;)V", &[JValue::from(&intent_obj)])?;
        Ok(())
    };

    if let Err(_e) = try_open(&mut env) {
        if env.exception_check().unwrap_or(false) {
            if let Ok(ex) = env.exception_occurred() {
                let _ = env.exception_clear();

                if let Ok(msg_obj) = env.call_method(ex, "toString", "()Ljava/lang/String;", &[]) {
                    let msg_jstr: JString = msg_obj.l().unwrap().into();
                    let msg_rust: String = env.get_string(&msg_jstr).unwrap().into();
                    info!("open_app_or_fallback: Java Exception: {}", msg_rust);
                }
            }
        }
        
        info!("open_app_or_fallback: Launch failed for {}, falling back to URL {}", package_name, fallback_url);
        let url_ptr = fallback_url.to_il2cpp_string();
        Application::OpenURL(url_ptr);
    }
}

pub fn get_activity(mut env: JNIEnv<'_>) -> Option<JObject<'_>> {
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
    let activities_map = JMap::from_env(&mut env, &activities).ok()?;

    // Get the first activity in the map
    let (_, activity_record) = activities_map
        .iter(&mut env)
        .ok()?
        .next(&mut env)
        .ok()??
        ;
    let activity = env
        .get_field(activity_record, "activity", "Landroid/app/Activity;")
        .ok()?
        .l()
        .ok()?;
    Some(activity)
}

pub fn get_device_api_level(env: *mut jni::sys::JNIEnv) -> i32 {
    let mut env = unsafe { JNIEnv::from_raw(env).unwrap() };
    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
        .unwrap()
        .i()
        .unwrap()
}

pub fn get_screen_dimensions(mut env: JNIEnv) -> (i32, i32) {
    let Some(activity) = get_activity(unsafe { env.unsafe_clone() }) else { return (0, 0) };

    let wm = env.call_method(activity, "getWindowManager", "()Landroid/view/WindowManager;", &[])
        .ok().and_then(|v| v.l().ok()).unwrap();
    let display = env.call_method(wm, "getDefaultDisplay", "()Landroid/view/Display;", &[])
        .ok().and_then(|v| v.l().ok()).unwrap();

    let dm_class = env.find_class("android/util/DisplayMetrics").unwrap();
    let dm = env.new_object(dm_class, "()V", &[]).unwrap();

    env.call_method(display, "getRealMetrics", "(Landroid/util/DisplayMetrics;)V", &[(&dm).into()]).unwrap();

    let width = env.get_field(&dm, "widthPixels", "I").unwrap().i().unwrap();
    let height = env.get_field(&dm, "heightPixels", "I").unwrap().i().unwrap();

    (width, height)
}

pub fn get_game_dir() -> PathBuf {
    let package_name = game_impl::get_package_name();
    game_impl::get_data_dir(&package_name)
}


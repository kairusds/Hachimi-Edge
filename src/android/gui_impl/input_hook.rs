#![allow(non_snake_case)]

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use egui::Vec2;
use jni::{
    objects::{JMap, JObject, JValue},
    sys::{jboolean, jint, JNI_TRUE},
    JNIEnv,
};

use crate::{core::{Error, Gui, Hachimi}, il2cpp::symbols::Thread};

use super::keymap;

const ACTION_DOWN: jint = 0;
const ACTION_UP: jint = 1;
const ACTION_MOVE: jint = 2;
const ACTION_POINTER_DOWN: jint = 5;
const ACTION_POINTER_UP: jint = 6;
const ACTION_HOVER_MOVE: jint = 7;
const ACTION_SCROLL: jint = 8;
const ACTION_MASK: jint = 0xff;
const ACTION_POINTER_INDEX_MASK: jint = 0xff00;
const ACTION_POINTER_INDEX_SHIFT: jint = 8;

const TOOL_TYPE_MOUSE: jint = 3;

const AXIS_VSCROLL: jint = 9;
const AXIS_HSCROLL: jint = 10;
const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(300);

static VOLUME_UP_PRESSED: AtomicBool = AtomicBool::new(false);
static VOLUME_DOWN_PRESSED: AtomicBool = AtomicBool::new(false);
static VOLUME_UP_LAST_TAP: once_cell::sync::Lazy<Arc<Mutex<Option<Instant>>>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));
static IME_REQUESTED: AtomicI32 = AtomicI32::new(-1);

static SCROLL_AXIS_SCALE: f32 = 10.0;

type NativeInjectEventFn = extern "C" fn(env: JNIEnv, obj: JObject, input_event: JObject, extra_param: jint) -> jboolean;
extern "C" fn nativeInjectEvent(mut env: JNIEnv, obj: JObject, input_event: JObject, extra_param: jint) -> jboolean {
    let motion_event_class = env.find_class("android/view/MotionEvent").unwrap();
    let key_event_class = env.find_class("android/view/KeyEvent").unwrap();

    if env.is_instance_of(&input_event, &motion_event_class).unwrap() {
        if !Gui::is_consuming_input_atomic() {
            return get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param);
        }

        let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
            return get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param);
        };

        let get_action_res = env.call_method(&input_event, "getAction", "()I", &[]).unwrap();
        let action = get_action_res.i().unwrap();
        let action_masked = action & ACTION_MASK;
        let pointer_index = (action & ACTION_POINTER_INDEX_MASK) >> ACTION_POINTER_INDEX_SHIFT;

        if pointer_index != 0 {
            return JNI_TRUE;
        }

        if action_masked == ACTION_SCROLL {
            let x = env.call_method(&input_event, "getAxisValue", "(I)F", &[AXIS_HSCROLL.into()])
                .unwrap()
                .f()
                .unwrap();
            let y = env.call_method(&input_event, "getAxisValue", "(I)F", &[AXIS_VSCROLL.into()])
                .unwrap()
                .f()
                .unwrap();
            gui.input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(x, y) * SCROLL_AXIS_SCALE,
                modifiers: egui::Modifiers::default(),
            });
        }
        else {
            // borrowing egui's touch phase enum
            let phase = match action_masked {
                ACTION_DOWN | ACTION_POINTER_DOWN => egui::TouchPhase::Start,
                ACTION_MOVE | ACTION_HOVER_MOVE => egui::TouchPhase::Move,
                ACTION_UP | ACTION_POINTER_UP => egui::TouchPhase::End,
                _ => return JNI_TRUE
            };

            // dumb and simple, no multi touch
            let real_x = env.call_method(&input_event, "getX", "()F", &[])
                .unwrap()
                .f()
                .unwrap();
            let real_y = env.call_method(&input_event, "getY", "()F", &[])
                .unwrap()
                .f()
                .unwrap();
            let tool_type = env.call_method(&input_event, "getToolType", "(I)I", &[0.into()])
                .unwrap()
                .i()
                .unwrap();

            let ppp = get_ppp(env, &gui);
            let x = real_x / ppp;
            let y = real_y / ppp;
            let pos = egui::Pos2 { x, y };

            match phase {
                egui::TouchPhase::Start => {
                    gui.input.events.push(egui::Event::PointerMoved(pos));
                    gui.input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default()
                    });
                },
                egui::TouchPhase::Move => {
                    gui.input.events.push(egui::Event::PointerMoved(pos));
                },
                egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                    gui.input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default()
                    });
                    if tool_type != TOOL_TYPE_MOUSE {
                        gui.input.events.push(egui::Event::PointerGone);
                    }
                }
            }
        }

        return JNI_TRUE;
    }
    else if env.is_instance_of(&input_event, &key_event_class).unwrap() {
        let action = env.call_method(&input_event, "getAction", "()I", &[])
            .unwrap()
            .i()
            .unwrap();
        let key_code = env.call_method(&input_event, "getKeyCode", "()I", &[])
            .unwrap()
            .i()
            .unwrap();
        let repeat_count = env.call_method(&input_event, "getRepeatCount", "()I", &[])
            .unwrap()
            .i()
            .unwrap();

        let pressed = action == ACTION_DOWN;
        let now = Instant::now();
        let other_atomic = match key_code {
            keymap::KEYCODE_VOLUME_UP => {
                VOLUME_UP_PRESSED.store(pressed, Ordering::Relaxed);

                if pressed && repeat_count == 0 {
                    if Hachimi::instance().config.load().hide_ingame_ui_hotkey && check_volume_up_double_tap(now) {
                        return JNI_TRUE; 
                    }
                }
                &VOLUME_DOWN_PRESSED
            }
            keymap::KEYCODE_VOLUME_DOWN => {
                VOLUME_DOWN_PRESSED.store(pressed, Ordering::Relaxed);

                if pressed {
                    reset_volume_up_tap_state();
                }
                &VOLUME_UP_PRESSED
            }
            _ => {
                if pressed && key_code == Hachimi::instance().config.load().android.menu_open_key {
                    let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
                        return get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param);
                    };
                    gui.toggle_menu();
                }
                if Hachimi::instance().config.load().hide_ingame_ui_hotkey && pressed
                    && key_code == Hachimi::instance().config.load().android.hide_ingame_ui_hotkey_bind {
                    Thread::main_thread().schedule(Gui::toggle_game_ui);
                }
                if Gui::is_consuming_input_atomic() {
                    let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
                        return get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param);
                    };

                    if let Some(key) = keymap::get_key(key_code) {
                        gui.input.events.push(egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed,
                            repeat: false,
                            modifiers: Default::default()
                        });
                    }

                    if pressed {
                        let c = env.call_method(&input_event, "getUnicodeChar", "()I", &[])
                            .unwrap()
                            .i()
                            .unwrap();
                        if c != 0 {
                            if let Some(c) = char::from_u32(c as _) {
                                gui.input.events.push(egui::Event::Text(c.to_string()));
                            }
                        }
                    }
                    return JNI_TRUE;
                }
                return get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param);
            }
        };

        if pressed && other_atomic.load(Ordering::Relaxed) {
            let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
                return get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param);
            };
            gui.toggle_menu();
        }
    }

    get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param)
}

fn get_ppp(mut env: JNIEnv, gui: &Gui) -> f32 {
    // SAFETY: view doesn't live past the lifetime of this function
    let Some(view) = get_view(unsafe { env.unsafe_clone() }) else {
        return gui.context.pixels_per_point();
    };
    let view_width = env.call_method(&view, "getWidth", "()I", &[]).unwrap().i().unwrap();
    let view_height = env.call_method(&view, "getHeight", "()I", &[]).unwrap().i().unwrap();
    let view_main_axis_size = if view_width < view_height { view_width } else { view_height };

    gui.context.zoom_factor() * (view_main_axis_size as f32 / gui.prev_main_axis_size as f32)
}

fn get_activity(mut env: JNIEnv) -> Option<JObject<'_>> {
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

fn get_view(mut env: JNIEnv) -> Option<JObject<'_>> {
    let activity = get_activity(unsafe { env.unsafe_clone() })?;
    let jni_window = env
        .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
        .ok()?
        .l()
        .ok()?;

    env.call_method(jni_window, "getDecorView", "()Landroid/view/View;", &[])
        .ok()?
        .l()
        .ok()
}

fn clear_jni_exception(env: &mut JNIEnv, context: &str) -> bool {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
        warn!("IME: cleared JNI exception at {}", context);
        return true;
    }
    false
}

pub(crate) fn set_ime_visible(visible: bool) {
    info!("IME set_visible={}", visible);
    crate::core::gui::set_ime_visible(visible);
    let Some(vm) = crate::android::main::java_vm() else {
        warn!("IME: JavaVM unavailable");
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        warn!("IME: attach_current_thread failed");
        return;
    };
    let Some(activity) = get_activity(unsafe { env.unsafe_clone() }) else {
        warn!("IME: get_activity failed");
        return;
    };
    if clear_jni_exception(&mut env, "get_activity") {
        return;
    }
    let Some(view) = get_view(unsafe { env.unsafe_clone() }) else {
        warn!("IME: get_view failed");
        return;
    };
    if clear_jni_exception(&mut env, "get_view") {
        return;
    }
    let target_view = match env
        .call_method(&activity, "getCurrentFocus", "()Landroid/view/View;", &[])
        .and_then(|v| v.l())
    {
        Ok(focus) if !focus.is_null() => focus,
        _ => view,
    };
    let _ = clear_jni_exception(&mut env, "getCurrentFocus");
    let context = activity;
    let Ok(context_class) = env.find_class("android/content/Context") else {
        warn!("IME: find Context class failed");
        return;
    };
    let Ok(service_name) = env
        .get_static_field(context_class, "INPUT_METHOD_SERVICE", "Ljava/lang/String;")
        .and_then(|v| v.l())
    else {
        warn!("IME: get INPUT_METHOD_SERVICE failed");
        return;
    };
    let Ok(imm) = env
        .call_method(
            &context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&service_name)],
        )
        .and_then(|v| v.l())
    else {
        warn!("IME: getSystemService failed");
        return;
    };
    if clear_jni_exception(&mut env, "getSystemService") {
        return;
    }
    if visible {
        let res = env.call_method(
            &imm,
            "showSoftInput",
            "(Landroid/view/View;I)Z",
            &[JValue::Object(&target_view), JValue::Int(2)],
        );
        let shown = res.ok().and_then(|v| v.z().ok()).unwrap_or(false);
        if !shown {
            let _ = env.call_method(
                &imm,
                "toggleSoftInput",
                "(II)V",
                &[JValue::Int(2), JValue::Int(1)],
            );
            warn!("IME: showSoftInput returned false, toggled instead");
        }
        let _ = clear_jni_exception(&mut env, "showSoftInput");
    } else {
        let Ok(window_token) = env
            .call_method(&target_view, "getWindowToken", "()Landroid/os/IBinder;", &[])
            .and_then(|v| v.l())
        else {
            warn!("IME: getWindowToken failed");
            return;
        };
        let res = env.call_method(
            &imm,
            "hideSoftInputFromWindow",
            "(Landroid/os/IBinder;I)Z",
            &[JValue::Object(&window_token), JValue::Int(0)],
        );
        if res.is_err() {
            warn!("IME: hideSoftInputFromWindow failed");
        }
        let _ = clear_jni_exception(&mut env, "hideSoftInputFromWindow");
    }
}

pub(crate) fn request_ime_visible(visible: bool) {
    let desired = if visible { 1 } else { 0 };
    IME_REQUESTED.store(desired, Ordering::Relaxed);
    Thread::main_thread().schedule(apply_ime_request);
}

fn apply_ime_request() {
    let desired = IME_REQUESTED.swap(-1, Ordering::Relaxed);
    if desired == 1 {
        set_ime_visible(true);
    } else if desired == 0 {
        set_ime_visible(false);
    }
}

fn reset_volume_up_tap_state() {
    let tap_state = &VOLUME_UP_LAST_TAP;
    if let Ok(mut guard) = tap_state.lock() {
        *guard = None;
    }
}

fn check_volume_up_double_tap(now: Instant) -> bool {
    let tap_state = &VOLUME_UP_LAST_TAP;
    let mut is_double_tap = false;

    let mut last_tap_time_guard = match tap_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Mutex poisoned: {:?}", poisoned);
            poisoned.into_inner()
        }
    };

    if let Some(last_time) = *last_tap_time_guard {
        let time_since_last_tap = now.duration_since(last_time);

        if time_since_last_tap <= DOUBLE_TAP_WINDOW {
            is_double_tap = true;
            *last_tap_time_guard = None;
            Thread::main_thread().schedule(Gui::toggle_game_ui);
        }else {
            *last_tap_time_guard = Some(now); 
        }
    }else {
        *last_tap_time_guard = Some(now);
    }

    is_double_tap
}

pub static mut NATIVE_INJECT_EVENT_ADDR: usize = 0;

fn init_internal() -> Result<(), Error> {
    let native_inject_event_addr = unsafe { NATIVE_INJECT_EVENT_ADDR };
    if native_inject_event_addr != 0 {
        info!("Hooking nativeInjectEvent");
        Hachimi::instance().interceptor.hook(unsafe { NATIVE_INJECT_EVENT_ADDR }, nativeInjectEvent as usize)?;
    }
    else {
        error!("native_inject_event_addr is null");
    }

    Ok(())
}

pub fn init() {
    init_internal().unwrap_or_else(|e| {
        error!("Init failed: {}", e);
    });
}

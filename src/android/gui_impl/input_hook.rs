#![allow(non_snake_case)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use egui::Vec2;
use jni::{objects::{JMap, JObject}, sys::{jboolean, jint, JNI_TRUE}, JNIEnv};

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
    let view = get_view(unsafe { env.unsafe_clone() });
    let view_width = env.call_method(&view, "getWidth", "()I", &[]).unwrap().i().unwrap();
    let view_height = env.call_method(&view, "getHeight", "()I", &[]).unwrap().i().unwrap();
    let view_main_axis_size = if view_width < view_height { view_width } else { view_height };

    gui.context.zoom_factor() * (view_main_axis_size as f32 / gui.prev_main_axis_size as f32)
}

fn get_view(mut env: JNIEnv) -> JObject<'_> {
    let activity_thread_class = env.find_class("android/app/ActivityThread").unwrap();
    let activity_thread = env
        .call_static_method(
            activity_thread_class,
            "currentActivityThread",
            "()Landroid/app/ActivityThread;",
            &[]
        )
        .unwrap()
        .l()
        .unwrap();
    let activities = env
        .get_field(activity_thread, "mActivities", "Landroid/util/ArrayMap;")
        .unwrap()
        .l()
        .unwrap();
    let activities_map = JMap::from_env(&mut env, &activities).unwrap();

    // Get the first activity in the map
    let (_, activity_record) = activities_map.iter(&mut env).unwrap().next(&mut env).unwrap().unwrap();
    let activity = env
        .get_field(activity_record, "activity", "Landroid/app/Activity;")
        .unwrap()
        .l()
        .unwrap();

    let jni_window = env
        .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
        .unwrap()
        .l()
        .unwrap();

    env
        .call_method(jni_window, "getDecorView", "()Landroid/view/View;", &[])
        .unwrap()
        .l()
        .unwrap()
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
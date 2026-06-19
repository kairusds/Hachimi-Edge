use std::{
    ptr::null_mut,
    sync::Mutex
};
use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        hook::{
            UnityEngine_CoreModule::{Component, GameObject, Object, Resources, Transform},
            UnityEngine_UI::Text,
            UnityEngine_UIModule::CanvasGroup,
            umamusume::{AudioManager, GallopUtil, ImageCommon, MasterCharacterSystemText::{self, CharacterSystemText}, Notification, PartsCharaMessageBase, SceneManager, TextCommon, UIManager}
        },
        symbols::{self, GCHandle, Thread},
        types::*
    }
};
use once_cell::sync::Lazy;

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct CueInfo {
    pub id: i32,
    pub type_: i32,
    pub name: *mut Il2CppString,
}

#[derive(Clone)]
pub struct CaptionData {
    pub text: String,
    pub cue_sheet: String,
    pub cue_id: i32,
    pub character_id: i32,
    pub voice_id: i32,
}

pub static CAPTION_REQUEST: Lazy<Mutex<Option<CaptionData>>> = Lazy::new(|| Mutex::new(None));

fn lookup_caption(chara_id: i32, cue_id: i32, cue_name: &str) -> Option<CaptionData> {
    let list = MasterCharacterSystemText::GetByCharaId(chara_id);
    if list.is_null() { return None; }

    if let Some(ilist) = symbols::IList::<*mut Il2CppObject>::new(list) {
        for item in ilist.iter() {
            if item.is_null() { continue; }

            let item_cue_id = CharacterSystemText::get_CueId(item);
            let item_cue_sheet_ptr = CharacterSystemText::get_CueSheet(item);
            if item_cue_sheet_ptr.is_null() { continue; }

            if item_cue_id == cue_id {
                let item_cue_sheet = unsafe { (*item_cue_sheet_ptr).as_utf16str().to_string() };
                if cue_name.starts_with(&item_cue_sheet) {
                    let text_ptr = CharacterSystemText::get_Text(item);
                    if text_ptr.is_null() { break; }
                    let voice_id = CharacterSystemText::get_VoiceId(item);

                    let orig_text = unsafe { (*text_ptr).as_utf16str().to_string() };
                    let clean_text = orig_text.replace("\n\n", " ").replace("\n", " ");

                    if !cue_name.contains("_home_") && !cue_name.contains("_tc_") &&
                        !cue_name.contains("_title_") && !cue_name.contains("_kakao_") &&
                        !cue_name.contains("_gacha_") && voice_id != 95001 &&
                        (chara_id < 9000 || voice_id == 95005 || voice_id == 95006 || voice_id == 70000)
                    {
                        let mut valid = true;
                        if cue_name.contains("_training_") && (item_cue_id < 29 || item_cue_id == 39) {
                            if !((voice_id >= 2030 && voice_id <= 2037) || voice_id >= 93000 || [8, 9, 12, 13].contains(&item_cue_id)) {
                            } else if voice_id == 20025 {
                                let scene_manager = SceneManager::instance();
                                if scene_manager.is_null() { break; }
                                if SceneManager::GetCurrentViewId(scene_manager) != 5901 {
                                    valid = false;
                                }
                            } else {
                                valid = false;
                            }
                        }

                        if valid {
                            return Some(CaptionData {
                                text: clean_text,
                                cue_sheet: item_cue_sheet,
                                cue_id: item_cue_id,
                                character_id: chara_id,
                                voice_id,
                            });
                        }
                    }
                    break;
                }
            }
        }
    }
    None
}

pub fn process_caption_request() {
    let request = match CAPTION_REQUEST.lock() {
        Ok(mut slot) => slot.take(),
        Err(poisoned) => {
            warn!("[captions] CAPTION_REQUEST mutex poisoned, recovering");
            poisoned.into_inner().take()
        }
    };
    let Some(caption_data) = request else { return; };

    let final_data = match lookup_caption(caption_data.character_id, caption_data.cue_id, &caption_data.cue_sheet) {
        Some(c) => c,
        None => return,
    };

    let am = AudioManager::instance();
    let length = if !am.is_null() {
        AudioManager::GetCueLength(am, final_data.cue_sheet.to_il2cpp_string(), final_data.cue_id)
    } else { 0.0 };
    let length = if length <= 0.0 { 3.0 } else { length };

    let mut caption_is_redundant = false;
    unsafe {
        let parts_type = PartsCharaMessageBase::type_object();
        if !parts_type.is_null() {
            let objects = Object::FindObjectsOfType(parts_type, false);
            if !objects.this.is_null() && objects.len() > 0 {
                for obj in objects.as_slice() {
                    if !obj.is_null() && PartsCharaMessageBase::get_IsPlaying(*obj) {
                        caption_is_redundant = true;
                        break;
                    }
                }
            }
        }
    }

    if !caption_is_redundant {
        let balloon = GameObject::Find(
            "/Gallop.GameSystem/SystemManagerRoot/SystemSingleton/UIManager/GameCanvas/MainCanvas/EpisodeCharacterView(Clone)/ContentsRoot/PartsEpisodeList/MidArea/BalloonRoot".to_il2cpp_string()
        );
        if !balloon.is_null() {
            caption_is_redundant = true;
        }
    }

    if caption_is_redundant {
        return;
    }

    let localized_text = Hachimi::instance().localized_data.load()
        .character_system_text_dict
        .get(&final_data.character_id)
        .and_then(|dict| dict.get(&final_data.voice_id))
        .cloned()
        .unwrap_or_else(|| final_data.text.clone());

    Captions::init();
    Captions::set_display_time(length);

    let config = Hachimi::instance().config.load();
    Captions::set_format(
        config.caption.caption_font_size,
        &config.caption.caption_color,
        &config.caption.caption_outline_size,
        &config.caption.caption_outline_color,
        config.caption.caption_pos_x,
        config.caption.caption_pos_y,
        config.caption.caption_bg_alpha,
    );
    Captions::show(&localized_text, config.caption.caption_lines_char_count);
}

struct CaptionState {
    handle: Option<GCHandle>,
    inited: bool,
    fade_id: u64,
    fade_start_time: Option<std::time::Instant>,
    display_time: f32,
    fade_out_time: f32,
}

impl CaptionState {
    fn notification(&self) -> *mut Il2CppObject {
        self.handle.as_ref().map_or(null_mut(), |h| h.target())
    }

    fn clear(&mut self) {
        self.handle = None;
        self.inited = false;
        self.fade_id = self.fade_id.wrapping_add(1);
    }

    fn set_notification(&mut self, obj: *mut Il2CppObject) {
        self.handle = None;
        self.fade_id = self.fade_id.wrapping_add(1);
        if !obj.is_null() {
            self.handle = Some(GCHandle::new(obj, false));
        }
    }
}

static STATE: Lazy<Mutex<CaptionState>> = Lazy::new(|| {
    Mutex::new(CaptionState {
        handle: None,
        inited: false,
        fade_id: 0,
        fade_start_time: None,
        display_time: 0.0,
        fade_out_time: 0.5,
    })
});

fn is_native_alive(obj: *mut Il2CppObject) -> bool {
    if obj.is_null() { return false; }
    Object::IsNativeObjectAlive(obj)
}

#[cfg(target_os = "windows")]
fn seh_guard<F: FnMut()>(mut f: F) {
    if microseh::try_seh(|| f()).is_err() {
        warn!("[captions] SEH exception caught, resetting state");
        if let Ok(mut st) = STATE.lock() { st.clear(); }
    }
}

#[cfg(not(target_os = "windows"))]
fn seh_guard<F: FnOnce()>(f: F) { f(); }

// Drop the mutex lock BEFORE making IL2CPP calls to prevent deadlock.
// Rust's Mutex is not re-entrant, if an IL2CPP callback tries to acquire
// STATE on the same thread, holding the lock here would deadlock.
fn init_impl() {
    // Phase 1: Check state under lock, extract what's needed, then release
    let (needs_init, _notif_check) = {
        let st = STATE.lock().unwrap();
        let notif = st.notification();
        let skip = st.inited && !notif.is_null() && is_native_alive(notif);
        (!skip, if skip { notif } else { null_mut() })
    };
    if !needs_init { return; }

    // Phase 2: All IL2CPP calls happen WITHOUT the lock held
    let mut st = STATE.lock().unwrap();
    st.clear();

    let ui_manager = UIManager::instance();
    if ui_manager.is_null() { return; }

    let mut canvas = UIManager::get_noticeCanvas(ui_manager);
    if canvas.is_null() {
        canvas = UIManager::get_systemCanvas(ui_manager);
    }
    if canvas.is_null() {
        canvas = UIManager::get_mainCanvas(ui_manager);
    }
    if canvas.is_null() { return; }

    let transform = Component::get_transform(canvas);
    if transform.is_null() { return; }

    let path = "UI/Parts/Notification".to_il2cpp_string();

    let go_type = GameObject::type_object();
    if go_type.is_null() { return; }

    let prefab = Resources::Load(path, go_type);
    if prefab.is_null() { return; }

    let inst = Object::Internal_CloneSingleWithParent(prefab, transform, false);
    if inst.is_null() { return; }

    let notif_type = Notification::type_object();
    if notif_type.is_null() { return; }

    let new_notif = GameObject::GetComponentInChildren(inst, notif_type, true);
    if new_notif.is_null() { return; }

    st.set_notification(new_notif);

    let go = Notification::get_gameObject(new_notif);
    if !go.is_null() {
        GameObject::SetActive(go, false);
        st.inited = true;
    }
    if !st.inited { st.clear(); }
}

fn show_impl(text: &str, line_char_count: i32) {
    let mut st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) {
        st.clear();
        return;
    }
    let my_fade_id = st.fade_id.wrapping_add(1);
    st.fade_id = my_fade_id;
    drop(st);

    let label = Notification::get__Label(notif);
    if label.is_null() { return; }

    let mut il2_text = text.to_il2cpp_string();
    if line_char_count > 0 {
        let wrapped = GallopUtil::LineHeadWrap(il2_text, line_char_count);
        if !wrapped.is_null() { il2_text = wrapped; }
    }

    Text::set_text(label, il2_text);

    let cg = Notification::get_canvasGroup(notif);
    if !cg.is_null() {
        CanvasGroup::set_alpha(cg, 1.0);
    }

    let go = Notification::get_gameObject(notif);
    if !go.is_null() {
        GameObject::SetActive(go, true);
    }

    let display_time = Notification::get__displayTime(notif);
    let fade_out_time = Notification::get__fadeOutTime(notif);

    {
        let mut st = STATE.lock().unwrap();
        st.fade_start_time = Some(std::time::Instant::now());
        st.display_time = display_time;
        st.fade_out_time = fade_out_time;
    }
    Thread::main_thread().schedule(fade_tick_global);
}

fn fade_tick_global() {
    let st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) { return; }

    let current_fade_id = st.fade_id;
    let start_time = match st.fade_start_time {
        Some(t) => t,
        None => return,
    };
    let display_time = st.display_time;
    let fade_out = st.fade_out_time;
    drop(st);

    {
        let st = STATE.lock().unwrap();
        if st.fade_id != current_fade_id { return; }
    }

    let elapsed = start_time.elapsed().as_secs_f32();
    let mut alpha = 1.0f32;
    let mut active = true;
    let mut done = false;

    if elapsed >= display_time + fade_out {
        alpha = 0.0;
        active = false;
        done = true;
    } else if elapsed >= display_time {
        let progress = (elapsed - display_time) / fade_out.max(0.001);
        alpha = 1.0 - progress.clamp(0.0, 1.0);
    }

    // Set alpha
    let cg = Notification::get_canvasGroup(notif);
    if !cg.is_null() {
        CanvasGroup::set_alpha(cg, alpha);
    }

    if !active {
        let go = Notification::get_gameObject(notif);
        if !go.is_null() {
            GameObject::SetActive(go, false);
        }
    }

    if !done {
        Thread::main_thread().schedule(fade_tick_global);
    }
}

fn set_display_time_impl(time: f32) {
    let st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) { return; }
    drop(st);

    Notification::set__displayTime(notif, time);
}

fn set_format_impl(
    font_size: i32,
    font_color: &str,
    outline_size: &str,
    outline_color: &str,
    pos_x: f32,
    pos_y: f32,
    bg_alpha: f32,
) {
    let st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) { return; }
    drop(st);

    let label = Notification::get__Label(notif);
    if label.is_null() { return; }

    Text::set_fontSize(label, font_size);
    Text::set_best_fit_max_size(label, font_size);

    if !font_color.is_empty() {
        if let Some(e) = symbols::parse_enum(
            symbols::get_runtime_type(c"umamusume.dll", c"Gallop", c"FontColorType"),
            font_color
        ) {
            let v = symbols::get_enum_int(e);
            TextCommon::set_FontColor(label, v);
        }
    }

    if !outline_size.is_empty() {
        if let Some(e) = symbols::parse_enum(
            symbols::get_runtime_type(c"umamusume.dll", c"Gallop", c"OutlineSizeType"),
            outline_size
        ) {
            let v = symbols::get_enum_int(e);
            TextCommon::set_OutlineSize(label, v);
        }
        TextCommon::UpdateOutline(label);
    }

    if !outline_color.is_empty() {
        if let Some(e) = symbols::parse_enum(
            symbols::get_runtime_type(c"umamusume.dll", c"Gallop", c"OutlineColorType"),
            outline_color
        ) {
            let v = symbols::get_enum_int(e);
            TextCommon::set_OutlineColor(label, v);
        }
        TextCommon::RebuildOutline(label);
    }

    let go = Notification::get_gameObject(notif);
    if !go.is_null() {
        let img_type = ImageCommon::type_object();
        if !img_type.is_null() {
            let bg = GameObject::GetComponentInChildren(go, img_type, true);
            if !bg.is_null() {
                ImageCommon::SetAlpha(bg, bg_alpha);
            }
        }
    }

    let cg = Notification::get_canvasGroup(notif);
    if cg.is_null() || !is_native_alive(cg) { return; }

    let cg_tr = Component::get_transform(cg);
    if cg_tr.is_null() { return; }

    let mut pos = Transform::get_position(cg_tr);
    pos.x = pos_x;
    pos.y = pos_y;
    Transform::set_position(cg_tr, pos);
}

fn cleanup_impl() {
    let mut st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) { return; }

    st.fade_id = st.fade_id.wrapping_add(1);
    drop(st);

    let cg = Notification::get_canvasGroup(notif);
    if !cg.is_null() {
        CanvasGroup::set_alpha(cg, 0.0);
    }

    let go = Notification::get_gameObject(notif);
    if !go.is_null() {
        GameObject::SetActive(go, false);
    }
}

pub struct Captions;

impl Captions {
    pub fn init() {
        seh_guard(init_impl);
    }

    pub fn show(text: &str, line_char_count: i32) {
        let text = text.to_owned();
        seh_guard(move || show_impl(&text, line_char_count));
    }

    pub fn set_display_time(time: f32) {
        seh_guard(move || set_display_time_impl(time));
    }

    pub fn set_format(
        font_size: i32,
        font_color: &str,
        outline_size: &str,
        outline_color: &str,
        pos_x: f32,
        pos_y: f32,
        bg_alpha: f32,
    ) {
        let fc = font_color.to_owned();
        let os = outline_size.to_owned();
        let oc = outline_color.to_owned();
        seh_guard(move || set_format_impl(font_size, &fc, &os, &oc, pos_x, pos_y, bg_alpha));
    }

    pub fn cleanup() {
        seh_guard(cleanup_impl);
    }

    pub fn reset() {
        if let Ok(mut st) = STATE.lock() { st.clear(); }
    }
}

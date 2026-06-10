use std::{ffi::CStr, os::raw::c_char, sync::{Mutex, Once}};
use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::{
    core::Hachimi,
    il2cpp::{symbols, types::*, ext::{Il2CppObjectExt, StringExt}},
};

#[repr(C)]
pub struct CueInfo {
    pub id: i32,
    pub type_: i32,
    pub name: *mut c_char,
}

#[derive(Clone)]
struct CaptionData {
    text: String,
    cue_sheet: String,
    cue_id: i32,
    character_id: i32,
    voice_id: i32,
}

static ACB_CAPTIONS: Lazy<Mutex<FnvHashMap<usize, CaptionData>>> = Lazy::new(|| Mutex::default());
static PLAYER_ACB: Lazy<Mutex<FnvHashMap<usize, usize>>> = Lazy::new(|| Mutex::default());
static ACTIVE_PLAYERS: Lazy<Mutex<FnvHashMap<usize, usize>>> = Lazy::new(|| Mutex::default());
static CAPTION_REQUESTS: Lazy<Mutex<Vec<CaptionData>>> = Lazy::new(|| Mutex::default());

static CUE_SHEET_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"_(?:9)*(\d{4})(?:\d{2})*_(\d{4})*_*(\d{2})*(?:\d{2})*$").unwrap());

static INIT: Once = Once::new();

type GetCueInfoByIdFn = extern "C" fn(acb: usize, id: i32, info: *mut CueInfo) -> bool;
static mut GETCUEINFOBYID_ORIG: usize = 0;
extern "C" fn get_cue_info_by_id(acb: usize, id: i32, info: *mut CueInfo) -> bool {
    let orig: GetCueInfoByIdFn = unsafe { std::mem::transmute(GETCUEINFOBYID_ORIG) };
    let res = orig(acb, id, info);
    if res && !info.is_null() && !unsafe { (*info).name }.is_null() {
        process_cue_info(acb, info);
    }
    res
}

type GetCueInfoByNameFn = extern "C" fn(acb: usize, name: *const c_char, info: *mut CueInfo) -> bool;
static mut GETCUEINFOBYNAME_ORIG: usize = 0;
extern "C" fn get_cue_info_by_name(acb: usize, name: *const c_char, info: *mut CueInfo) -> bool {
    let orig: GetCueInfoByNameFn = unsafe { std::mem::transmute(GETCUEINFOBYNAME_ORIG) };
    let res = orig(acb, name, info);
    if res && !info.is_null() && !unsafe { (*info).name }.is_null() {
        process_cue_info(acb, info);
    }
    res
}

fn process_cue_info(acb: usize, info: *mut CueInfo) {
    if !Hachimi::instance().config.load().caption.caption_enable { return; }
    ACB_CAPTIONS.lock().unwrap().remove(&acb);

    let cue_name = unsafe { CStr::from_ptr((*info).name).to_string_lossy().to_string() };
    if let Some(caps) = CUE_SHEET_REGEX.captures(&cue_name) {
        let mut chara_id_str = caps.get(1).map_or("", |m| m.as_str());
        if let (Some(m2), Some(m3)) = (caps.get(2), caps.get(3)) {
            if m3.as_str() == "01" {
                chara_id_str = m2.as_str();
            }
        }
        if let Ok(chara_id) = chara_id_str.parse::<i32>() {
            let image = match symbols::get_assembly_image(c"umamusume.dll") {
                Ok(i) => i,
                Err(_) => return
            };
            let master_class = match symbols::get_class(image, c"Gallop", c"MasterCharacterSystemText") {
                Ok(c) => c,
                Err(_) => return
            };
            let get_by_chara_id_addr = symbols::get_method_addr_cached(master_class, c"GetByCharaId", 1);
            if get_by_chara_id_addr == 0 { return; }
            let get_by_chara_id: extern "C" fn(i32) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_by_chara_id_addr) };

            let list = get_by_chara_id(chara_id);
            if !list.is_null() {
                if let Some(ilist) = crate::il2cpp::symbols::IList::<*mut crate::il2cpp::types::Il2CppObject>::new(list) {
                    let cue_id = unsafe { (*info).id };
                    for item in ilist.iter() {
                        if item.is_null() { continue; }
                        let item_klass = unsafe { (*item).klass() };
                        let cue_id_field = symbols::get_field_from_name(item_klass, c"CueId");
                        let cue_sheet_field = symbols::get_field_from_name(item_klass, c"CueSheet");
                        if cue_id_field.is_null() || cue_sheet_field.is_null() { continue; }

                        let item_cue_id = symbols::get_field_value::<i32>(item, cue_id_field);
                        let item_cue_sheet_ptr = symbols::get_field_object_value::<Il2CppString>(item, cue_sheet_field);

                        if item_cue_id == cue_id && !item_cue_sheet_ptr.is_null() {
                            use crate::il2cpp::ext::Il2CppStringExt;
                            let item_cue_sheet = unsafe { (*item_cue_sheet_ptr).as_utf16str().to_string() };
                            if cue_name.starts_with(&item_cue_sheet) {
                                let text_field = symbols::get_field_from_name(item_klass, c"Text");
                                let voice_id_field = symbols::get_field_from_name(item_klass, c"VoiceId");
                                if text_field.is_null() || voice_id_field.is_null() { break; }

                                let text_ptr = symbols::get_field_object_value::<Il2CppString>(item, text_field);
                                let voice_id = symbols::get_field_value::<i32>(item, voice_id_field);

                                if !text_ptr.is_null() {
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
                                                let Ok(scene_manager_class) = symbols::get_class(image, c"Gallop", c"SceneManager") else { break; };
                                                let Some(singleton) = symbols::SingletonLike::new(scene_manager_class) else { break; };
                                                let scene_manager = singleton.instance();
                                                if scene_manager.is_null() { break; }
                                                let get_view_id_addr = symbols::get_method_addr_cached(scene_manager_class, c"GetCurrentViewId", 0);
                                                if get_view_id_addr == 0 { break; }
                                                let get_view_id: extern "C" fn(*mut Il2CppObject) -> i32 = unsafe { std::mem::transmute(get_view_id_addr) };
                                                if get_view_id(scene_manager) != 5901 {
                                                    valid = false;
                                                }
                                            } else {
                                                valid = false;
                                            }
                                        }

                                        if valid {
                                            ACB_CAPTIONS.lock().unwrap().insert(acb, CaptionData {
                                                text: clean_text,
                                                cue_sheet: item_cue_sheet,
                                                cue_id: item_cue_id,
                                                character_id: chara_id,
                                                voice_id,
                                            });
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

type SetCueIdFn = extern "C" fn(player: usize, acb: usize, id: i32);
static mut SETCUEID_ORIG: usize = 0;
extern "C" fn set_cue_id(player: usize, acb: usize, id: i32) {
    let orig: SetCueIdFn = unsafe { std::mem::transmute(SETCUEID_ORIG) };
    if ACB_CAPTIONS.lock().unwrap().contains_key(&acb) {
        PLAYER_ACB.lock().unwrap().insert(player, acb);
    }
    orig(player, acb, id)
}

type StartFn = extern "C" fn(player: usize) -> u32;
static mut START_ORIG: usize = 0;
fn process_caption_requests() {
    let mut requests = CAPTION_REQUESTS.lock().unwrap();
    for caption_data in requests.drain(..) {
        let length = (|| -> Option<f32> {
            let image = symbols::get_assembly_image(c"umamusume.dll").ok()?;
            let audio_manager_class = symbols::get_class(image, c"Gallop", c"AudioManager").ok()?;
            let audio_manager = symbols::SingletonLike::new(audio_manager_class)?.instance();
            if audio_manager.is_null() { return None; }
            if !crate::il2cpp::hook::UnityEngine_CoreModule::Object::IsNativeObjectAlive(audio_manager) { return None; }

            let get_cue_length_method = symbols::get_method_cached(audio_manager_class, c"GetCueLength", 2).ok()?;
            let mut exc = std::ptr::null_mut();
            let mut params = [
                caption_data.cue_sheet.to_il2cpp_string() as *mut std::ffi::c_void,
                &caption_data.cue_id as *const _ as *mut std::ffi::c_void
            ];

            let res = crate::il2cpp::api::il2cpp_runtime_invoke(
                get_cue_length_method,
                audio_manager as *mut std::ffi::c_void,
                params.as_mut_ptr(),
                &mut exc
            );

            if !exc.is_null() || res.is_null() { return None; }
            Some(unsafe { *(crate::il2cpp::api::il2cpp_object_unbox(res) as *mut f32) })
        })().unwrap_or(3.0);

        let localized_text = Hachimi::instance().localized_data.load()
            .character_system_text_dict
            .get(&caption_data.character_id)
            .and_then(|dict| dict.get(&caption_data.voice_id))
            .cloned()
            .unwrap_or_else(|| caption_data.text.clone());

        crate::core::captions::Captions::init();
        crate::core::captions::Captions::set_display_time(length);

        let config = Hachimi::instance().config.load();
        crate::core::captions::Captions::set_format(
            config.caption.caption_font_size,
            &config.caption.caption_color,
            &config.caption.caption_outline_size,
            &config.caption.caption_outline_color,
            config.caption.caption_pos_x,
            config.caption.caption_pos_y,
            config.caption.caption_bg_alpha,
        );
        crate::core::captions::Captions::show(&localized_text, config.caption.caption_lines_char_count);
    }
}

extern "C" fn start(player: usize) -> u32 {
    let orig: StartFn = unsafe { std::mem::transmute(START_ORIG) };

    let acb = PLAYER_ACB.lock().unwrap().get(&player).cloned();

    if let Some(acb) = acb {
        let caption_data = ACB_CAPTIONS.lock().unwrap().get(&acb).cloned();

        if let Some(caption_data) = caption_data {
            ACTIVE_PLAYERS.lock().unwrap().insert(player, acb);

            CAPTION_REQUESTS.lock().unwrap().push(caption_data);
            crate::il2cpp::symbols::Thread::main_thread().schedule(process_caption_requests);
        }
    }

    orig(player)
}

type StopFn = extern "C" fn(player: usize);
static mut STOP_ORIG: usize = 0;
extern "C" fn stop(player: usize) {
    let orig: StopFn = unsafe { std::mem::transmute(STOP_ORIG) };
    orig(player);
    clear_active_player(player);
}

type StopWithoutReleaseTimeFn = extern "C" fn(player: usize);
static mut STOPWITHOUTRELEASETIME_ORIG: usize = 0;
extern "C" fn stop_without_release_time(player: usize) {
    let orig: StopWithoutReleaseTimeFn = unsafe { std::mem::transmute(STOPWITHOUTRELEASETIME_ORIG) };
    orig(player);
    clear_active_player(player);
}

type PauseFn = extern "C" fn(player: usize, sw: bool);
static mut PAUSE_ORIG: usize = 0;
extern "C" fn pause(player: usize, sw: bool) {
    let orig: PauseFn = unsafe { std::mem::transmute(PAUSE_ORIG) };
    orig(player, sw);
    if !sw {
        clear_active_player(player);
    }
}

type ReleaseFn = extern "C" fn(acb: usize);
static mut RELEASE_ORIG: usize = 0;
extern "C" fn release(acb: usize) {
    ACB_CAPTIONS.lock().unwrap().remove(&acb);
    let orig: ReleaseFn = unsafe { std::mem::transmute(RELEASE_ORIG) };
    orig(acb);
}

fn clear_active_player(player: usize) {
    if let Some(acb) = ACTIVE_PLAYERS.lock().unwrap().remove(&player) {
        ACB_CAPTIONS.lock().unwrap().remove(&acb);
        crate::core::captions::Captions::cleanup();
    }
}

pub fn init(handle: usize) {
    INIT.call_once(|| {
        info!("Initializing criware hooks");
        let hachimi = Hachimi::instance();
        unsafe {
            let get_cue_info_by_id_addr = crate::core::utils::get_proc_address(handle, c"criAtomExAcb_GetCueInfoById");
            let get_cue_info_by_name_addr = crate::core::utils::get_proc_address(handle, c"criAtomExAcb_GetCueInfoByName");
            let release_addr = crate::core::utils::get_proc_address(handle, c"criAtomExAcb_Release");
            let set_cue_id_addr = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_SetCueId");
            let start_addr = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_Start");
            let stop_addr = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_Stop");
            let stop_without_release_time_addr = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_StopWithoutReleaseTime");
            let pause_addr = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_Pause");

            if get_cue_info_by_id_addr != 0 {
                GETCUEINFOBYID_ORIG = hachimi.interceptor.hook(get_cue_info_by_id_addr, get_cue_info_by_id as *const () as usize).unwrap();
            }
            if get_cue_info_by_name_addr != 0 {
                GETCUEINFOBYNAME_ORIG = hachimi.interceptor.hook(get_cue_info_by_name_addr, get_cue_info_by_name as *const () as usize).unwrap();
            }
            if release_addr != 0 {
                RELEASE_ORIG = hachimi.interceptor.hook(release_addr, release as *const () as usize).unwrap();
            }
            if set_cue_id_addr != 0 {
                SETCUEID_ORIG = hachimi.interceptor.hook(set_cue_id_addr, set_cue_id as *const () as usize).unwrap();
            }
            if start_addr != 0 {
                START_ORIG = hachimi.interceptor.hook(start_addr, start as *const () as usize).unwrap();
            }
            if stop_addr != 0 {
                STOP_ORIG = hachimi.interceptor.hook(stop_addr, stop as *const () as usize).unwrap();
            }
            if stop_without_release_time_addr != 0 {
                STOPWITHOUTRELEASETIME_ORIG = hachimi.interceptor.hook(stop_without_release_time_addr, stop_without_release_time as *const () as usize).unwrap();
            }
            if pause_addr != 0 {
                PAUSE_ORIG = hachimi.interceptor.hook(pause_addr, pause as *const () as usize).unwrap();
            }
        }
    });
}
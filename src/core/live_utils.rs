use crate::{
    il2cpp::{
        api::il2cpp_runtime_invoke,
        ext::Il2CppObjectExt,
        hook::{
            umamusume::{AudioManager, Director, LiveTimeController, LiveViewController, SceneManager},
            Cute_Cri_Assembly::{AtomSourceEx, CuteAudioSource, CuteAudioSourcePool},
            CriMw_CriWare_Runtime::CriAtomExPlayer,
        },
        symbols::{Array, IList, get_field_from_name, get_field_object_value, get_method_cached}, types::*
    }
};

use std::{ffi::c_void, ptr::null_mut, sync::atomic::{AtomicBool, Ordering}};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CriAtomExPlayback {
    pub id: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AudioPlayback {
    pub criAtomExPlayback: CriAtomExPlayback,
    pub isError: bool,
    pub soundGroup: i32,
    pub is3dSound: bool,
    pub atomSourceListIndex: i32,
    pub cueSheetName: *mut Il2CppString,
    pub cueName: *mut Il2CppString,
    pub cueId: i32,
}

static DRAG_WAS_PAUSED: AtomicBool = AtomicBool::new(false);
static DRAG_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

fn get_live_view_controller() -> Option<*mut Il2CppObject> {
    let scene_manager = SceneManager::instance();
    if scene_manager.is_null() { return None; }

    let vc = SceneManager::GetCurrentViewController(scene_manager);
    if vc.is_null() { return None; }

    let name = unsafe { std::ffi::CStr::from_ptr((*(*vc).klass()).name) }.to_string_lossy();
    if name == "LiveViewController" {
        Some(vc)
    } else {
        None
    }
}

pub fn begin_live_drag() {
    let director = Director::instance();
    let was_paused = if director.is_null() {
        true
    } else {
        Director::is_live_paused()
    };

    DRAG_WAS_PAUSED.store(was_paused, Ordering::Release);

    if !director.is_null() && !was_paused {
        if let Some(lvc) = get_live_view_controller() {
            LiveViewController::PauseLive(lvc);
        } else {
            Director::PauseLive(director, true);
        }
    }

    DRAG_IN_PROGRESS.store(true, Ordering::Release);
}

pub fn end_live_drag() {
    let director = Director::instance();
    let was_paused = !DRAG_WAS_PAUSED.load(Ordering::Acquire);

    if !director.is_null() && was_paused {
        if let Some(lvc) = get_live_view_controller() {
            LiveViewController::ResumeLive(lvc);
        } else if !director.is_null() {
            Director::PauseLive(director, false);
        }
    }

    DRAG_IN_PROGRESS.store(false, Ordering::Release);
    DRAG_WAS_PAUSED.store(false, Ordering::Release);
}

pub fn reset_live_drag_state() {
    DRAG_WAS_PAUSED.store(false, Ordering::Release);
    DRAG_IN_PROGRESS.store(false, Ordering::Release);
}

unsafe fn process_playback(
    playback: &mut AudioPlayback,
    audio_ctrl_dict: *mut Il2CppObject,
    target_time: f32
) {
    let dict_class = (*audio_ctrl_dict).klass();
    let get_item_method = match get_method_cached(dict_class, c"get_Item", 1) {
        Ok(m) => m,
        Err(_) => return
    };

    let mut key = playback.soundGroup;
    let mut get_item_params: [*mut c_void; 1] = [&mut key as *mut _ as *mut c_void];
    let mut exc = null_mut();
    let audio_ctrl = il2cpp_runtime_invoke(
        get_item_method, audio_ctrl_dict as *mut c_void,
        get_item_params.as_mut_ptr(), &mut exc
    );
    if !exc.is_null() || audio_ctrl.is_null() { return; }
    let audio_ctrl = audio_ctrl as *mut Il2CppObject;

    let pool_field = get_field_from_name((*audio_ctrl).klass(), c"pool");
    let pool = get_field_object_value::<Il2CppObject>(audio_ctrl, pool_field);
    if pool.is_null() { return; }

    let source_list = CuteAudioSourcePool::get_sourceList(pool);
    if source_list.is_null() { return; }

    let Some(list) = IList::<*mut Il2CppObject>::new(source_list) else { return; };
    let count = list.count();
    let mut cute_audio_source: *mut Il2CppObject = null_mut();

    for i in 0..count {
        let obj = list.get(i).unwrap_or(null_mut());
        if !obj.is_null() {
            if CuteAudioSource::IsSamePlaybackId(obj, *playback) {
                cute_audio_source = obj;
                break;
            }
        }
    }

    if cute_audio_source.is_null() { return; }

    let source_list2 = CuteAudioSource::get_sourceList(cute_audio_source);
    if source_list2.is_null() { return; }

    let using_index = CuteAudioSource::get_usingIndex(cute_audio_source);

    let Some(list2) = IList::<*mut Il2CppObject>::new(source_list2) else { return; };
    let atom_source = list2.get(using_index).unwrap_or(null_mut());
    if atom_source.is_null() { return; }

    let player = AtomSourceEx::get_player(atom_source);
    if player.is_null() { return; }

    CriAtomExPlayer::StopWithoutReleaseTime(player);
    CriAtomExPlayer::SetStartTime(player, (target_time * 1000.0).round() as i64);

    let new_playback = CriAtomExPlayer::Start(player);
    if new_playback.id != 0 {
        CriAtomExPlayer::Update(player, new_playback);

        CriAtomExPlayer::Pause(player, true);

        playback.criAtomExPlayback = new_playback;

        AtomSourceEx::set_Playback(atom_source, new_playback);
    }
}

pub fn move_live_playback(target_time: f32) {
    let director = Director::instance();
    if director.is_null() { return; }

    let dragging = DRAG_IN_PROGRESS.load(Ordering::Acquire);
    let was_paused = if dragging {
        true
    } else {
        Director::is_live_paused()
    };

    if !dragging && !was_paused {
        Director::PauseLive(director, true);
    }

    Director::set__liveCurrentTime(director, target_time);

    let time_controller = Director::get_LiveTimeController(director);

    if !time_controller.is_null() {
        LiveTimeController::set__elapsedTime_TC(time_controller, target_time);
        LiveTimeController::set_CurrentTime_TC(time_controller, target_time);
    }

    let audio_manager = AudioManager::instance();

    if !audio_manager.is_null() {
        let cri_audio_manager = AudioManager::get_CriAudioManager();

        if !cri_audio_manager.is_null() {
            let audio_ctrl_dict_field = get_field_from_name(
                unsafe { (*cri_audio_manager).klass() }, c"audioCtrlDict"
            );
            let audio_ctrl_dict = get_field_object_value::<Il2CppObject>(
                cri_audio_manager, audio_ctrl_dict_field
            );

            if !audio_ctrl_dict.is_null() {
                let mut song_playback = AudioManager::get__songPlayback(audio_manager);

                unsafe { process_playback(&mut song_playback, audio_ctrl_dict, target_time); }

                AudioManager::set__songPlayback(audio_manager, song_playback);

                let song_chara_playbacks = AudioManager::get__songCharaPlaybacks(audio_manager);
                if !song_chara_playbacks.is_null() {
                    let chara_playbacks = Array::<AudioPlayback>::from(song_chara_playbacks);
                    unsafe {
                        let slice = chara_playbacks.as_slice();
                        for i in 0..slice.len() {
                            process_playback(&mut slice[i], audio_ctrl_dict, target_time);
                        }
                    }
                }
            }
        } else {
            warn!("get_CriAudioManager returned null! Skipping audio sync.");
        }
    }

    if !dragging && !was_paused {
        Director::PauseLive(director, false);
    }
}
use std::sync::atomic::Ordering;
use crate::{
    core::{Hachimi, captions, game::Region, gui, utils::{race_seek_seh, race_seek_stage}},
    il2cpp::{
        hook::Cute_Cri_Assembly::{
            AudioPlayback::{self, AudioPlayback_t},
            AtomSourceEx,
        },
        ext::Il2CppStringExt,
        symbols::{get_method_addr, get_field_from_name, Array, SingletonLike, Thread},
        types::*
    }
};
use super::{
    RaceManager,
    RaceBGMController,
    RaceSoundReplay,
};

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Category {
    BGM = 0,
    SE = 1,
    VOICE = 2,
    JIKKYO = 3,
    LIVE = 4
}

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

static mut GET_CRIAUDIOMANAGER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_CriAudioManager, GET_CRIAUDIOMANAGER_ADDR, *mut Il2CppObject,);

def_field_value_accessors!(get__songPlayback, set__songPlayback, _SONGPLAYBACK_FIELD, AudioPlayback_t);
def_field_object_accessors!(get__songCharaPlaybacks, set__songCharaPlaybacks, _SONGCHARAPLAYBACKS_FIELD, Il2CppArray);
def_field_value_accessors!(get__bgmPlayback, set__bgmPlayback, _BGMPLAYBACK_FIELD, AudioPlayback_t);
def_field_object_accessors!(get__atomSourceArrayBGM, set__atomSourceArrayBGM, _ATOMSOURCEARRAYBGM_FIELD, Il2CppArray);

def_method_wrapper_fn!(GetCueLength, GET_CUE_LENGTH_ADDR, f32, this: *mut Il2CppObject, cue_sheet: *mut Il2CppString, cue_id: i32);
def_method_wrapper_fn!(GetVolume, GET_VOLUME_ADDR, f32, category: Category);

pub fn race_slider_music_base() -> bool {
    let audio_manager = instance();
    if audio_manager.is_null() { return false; }

    let mut bgm = get__bgmPlayback(audio_manager);
    if bgm.criAtomExPlayback.id == 0 { return false; }

    let mut num_samples: i64 = 0;
    let mut sampling_rate: i32 = 0;
    if !AudioPlayback::GetNumPlayedSamples(&mut bgm, &mut num_samples, &mut sampling_rate) {
        return false;
    }
    if sampling_rate <= 0 { return false; }

    let secs = num_samples as f32 / sampling_rate as f32;
    if !secs.is_finite() || secs < 0.0 { return false; }

    gui::RACE_SLIDER_MUSIC_TIME.store(secs.to_bits(), Ordering::Release);
    true
}

// public Void PlayBgmFromName(ref String cueName, Boolean isLoop, Single volume, Single fadeInTime, Single fadeOutTime, Single startTime, Boolean isCrossFade, AutoStopType stopType)
def_method_wrapper_fn!(
    PlayBgmFromName, PLAY_BGM_FROM_NAME_ADDR, (),
    this: *mut Il2CppObject, cue_name: *mut *mut Il2CppString, is_loop: bool, volume: f32,
    fade_in_time: f32, fade_out_time: f32, start_time: f32, is_cross_fade: bool, stop_type: i32
);

pub fn play_race_bgm_cue(cue_name: *mut Il2CppString, position_secs: f32, bgm_volume: f32) -> bool {
    let audio_manager = instance();
    if audio_manager.is_null() { return false; }
    if cue_name.is_null() { return false; }

    let position = if position_secs.is_finite() && position_secs > 0.0 { position_secs } else { 0.0 };
    let volume = if bgm_volume.is_finite() && bgm_volume >= 0.0 { bgm_volume } else { 1.0 };
    let mut cue_name = cue_name;
    PlayBgmFromName(
        audio_manager, &mut cue_name, true, volume, 0.1, 0.1, position, false, 0
    );
    true
}

pub fn resync_race_music(race_manager: *mut Il2CppObject, target_time: f32) -> bool {
    let race_sound = RaceManager::get_RaceSound(race_manager);
    if !RaceSoundReplay::is_replay_sound(race_sound) { return true; }

    let bgm_controller = RaceSoundReplay::get_BGMController(race_sound);
    if !RaceBGMController::is_bgm_controller(bgm_controller) { return true; }

    race_seek_seh(|| {
        race_seek_stage(12); // music_volume
        let bgm_volume = RaceSoundReplay::GetBGMVolume(race_sound);
        let second_start = RaceBGMController::get_secondBgmStartTime(bgm_controller);
        let race_base = f32::from_bits(gui::RACE_SLIDER_DRAG_START_TIME.load(Ordering::Acquire));
        let music_valid = gui::RACE_SLIDER_MUSIC_VALID.load(Ordering::Acquire);
        let music_base = f32::from_bits(gui::RACE_SLIDER_MUSIC_TIME.load(Ordering::Acquire));

        let audio_manager = instance();
        if audio_manager.is_null() { return; }

        let sources_ptr = get__atomSourceArrayBGM(audio_manager);
        if sources_ptr.is_null() { return; }
        let sources: Array<*mut Il2CppObject> = Array::from(sources_ptr);

        race_seek_stage(13); // music_sweep
        for source in unsafe { sources.as_slice() }.iter() {
            if source.is_null() { continue; }
            if !AtomSourceEx::get_IsInUse(*source) { continue; }
            AtomSourceEx::Stop(*source, 0.0, 0);
        }

        race_seek_stage(14); // music_play_cue
        if second_start.is_finite() && second_start > 0.0 && target_time >= second_start {
            let position = if race_base >= second_start && music_valid {
                music_base + (target_time - race_base)
            } else {
                target_time - second_start
            };
            play_race_bgm_cue(RaceBGMController::get_secondBgmCueName(bgm_controller), position, bgm_volume);
            RaceBGMController::set_isRequestFirstBGM(bgm_controller, true);
            RaceBGMController::set_isStoppedFirstBGM(bgm_controller, true);
            RaceBGMController::set_isPlayedSecondBGM(bgm_controller, true);
        } else {
            let delay = RaceBGMController::get_firstBGMDelayTime(bgm_controller);
            let first_stop = RaceBGMController::get_firstBgmStopTime(bgm_controller);
    
            if target_time >= delay {
                let position = if race_base < first_stop && music_valid {
                    music_base + (target_time - race_base)
                } else {
                    target_time - delay
                };
                play_race_bgm_cue(RaceBGMController::get_firstBgmCueName(bgm_controller), position, bgm_volume);
                RaceBGMController::set_isRequestFirstBGM(bgm_controller, true);
                RaceBGMController::set_isStoppedFirstBGM(bgm_controller, false);
            } else {
                RaceBGMController::set_isRequestFirstBGM(bgm_controller, false);
                RaceBGMController::set_isStoppedFirstBGM(bgm_controller, false);
            }
            RaceBGMController::set_isPlayedSecondBGM(bgm_controller, false);
        }

        if Hachimi::instance().game.region == Region::Japan || Hachimi::instance().game.region == Region::Taiwan {
            let trigger_start = RaceBGMController::get_firstTriggerBgmPlayStartTime(bgm_controller);
            RaceBGMController::set_isPlayedFirstTriggerBgm(bgm_controller, target_time >= trigger_start);
        }

        race_seek_stage(0); // idle
    })
}

// Cute.Cri.Audio RequestCueInfo
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct RequestCueInfo {
    pub CueSheetName: *mut Il2CppString,
    pub CueName: *mut Il2CppString,
    pub CueId: i32,
}

// Cute.Cri SoundGroup
#[derive(Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum SoundGroup {
    Bgm = 0,
    Se = 1,
    Voice = 2,
}

// private AudioPlayback PlayInternal(SoundGroup group, RequestCueInfo cueInfo, PlayParameters playParam, AutoStopType stopType) { }
type PlayInternalFn = extern "C" fn(this: *mut Il2CppObject, group: SoundGroup,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback_t;
extern "C" fn PlayInternal(this: *mut Il2CppObject, group: SoundGroup,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback_t {
    let result = get_orig_fn!(PlayInternal, PlayInternalFn)(this, group, cue_info, play_param, stop_type);

    if group == SoundGroup::Voice && !cue_info.is_null() && Hachimi::instance().config.load().caption.caption_enable {
        let cue_sheet_ptr = unsafe { *cue_info }.CueSheetName;
        if !cue_sheet_ptr.is_null() {
            let cue_sheet_ptr = unsafe { *cue_info }.CueSheetName;
            let cue_sheet = if !cue_sheet_ptr.is_null() {
                unsafe { &*cue_sheet_ptr }.as_utf16str().to_string()
            } else {
                String::new()
            };

            let cue_name_ptr = unsafe { *cue_info }.CueName;
            let cue_name = if !cue_name_ptr.is_null() {
                unsafe { &*cue_name_ptr }.as_utf16str().to_string()
            } else {
                String::new()
            };

            let cue_id = unsafe { *cue_info }.CueId;

            debug!("[captions] PlayInternal Voice: cue_sheet={}, name='{}', id={}", cue_sheet, cue_name, cue_id);

            if let Some(last) = cue_sheet.rsplit('_').next() {
                if last.len() >= 6 {
                    if let Ok(chara_id) = last[..4].parse::<i32>() {
                        let caption_data = captions::CaptionData {
                            text: String::new(), 
                            cue_sheet: cue_sheet.clone(),
                            cue_id,
                            character_id: chara_id,
                            voice_id: 0,
                        };

                        match captions::CAPTION_REQUEST.lock() {
                            Ok(mut slot) => *slot = Some(caption_data),
                            Err(poisoned) => {
                                warn!("[captions] CAPTION_REQUEST mutex poisoned, recovering...");
                                *poisoned.into_inner() = Some(caption_data);
                            }
                        }

                        Thread::main_thread().schedule(captions::process_caption_request);
                    }
                }
            }
        }
    }

    result
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, AudioManager);

    let play_internal_addr = get_method_addr(AudioManager, c"PlayInternal", 4);
    new_hook!(play_internal_addr, PlayInternal);

    unsafe {
        CLASS = AudioManager;
        GET_CRIAUDIOMANAGER_ADDR = get_method_addr(AudioManager, c"get_CriAudioManager", 0);
        GET_CUE_LENGTH_ADDR = get_method_addr(AudioManager, c"GetCueLength", 2);
        PLAY_BGM_FROM_NAME_ADDR = get_method_addr(AudioManager, c"PlayBgmFromName", 8);
        GET_VOLUME_ADDR = get_method_addr(AudioManager, c"GetVolume", 1);

        _SONGPLAYBACK_FIELD = get_field_from_name(AudioManager, c"_songPlayback");
        _SONGCHARAPLAYBACKS_FIELD = get_field_from_name(AudioManager, c"_songCharaPlaybacks");
        _BGMPLAYBACK_FIELD = get_field_from_name(AudioManager, c"_bgmPlayback");
        _ATOMSOURCEARRAYBGM_FIELD = get_field_from_name(AudioManager, c"_atomSourceArrayBGM");
    }
}

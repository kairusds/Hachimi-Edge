use crate::{
    core::{Hachimi, captions, live_utils::AudioPlayback},
    il2cpp::{
        ext::Il2CppStringExt,
        symbols::{get_method_addr, get_field_from_name, SingletonLike, Thread},
        types::*
    }
};

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

def_field_value_accessors!(get__songPlayback, set__songPlayback, _SONGPLAYBACK_FIELD, crate::core::live_utils::AudioPlayback);
def_field_object_accessors!(get__songCharaPlaybacks, set__songCharaPlaybacks, _SONGCHARAPLAYBACKS_FIELD, Il2CppArray);

static mut GET_CUE_LENGTH_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetCueLength, GET_CUE_LENGTH_ADDR, f32, this: *mut Il2CppObject, cue_sheet: *mut Il2CppString, cue_id: i32);

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
) -> AudioPlayback;
extern "C" fn PlayInternal(this: *mut Il2CppObject, group: SoundGroup,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback {
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
        _SONGPLAYBACK_FIELD = get_field_from_name(AudioManager, c"_songPlayback");
        _SONGCHARAPLAYBACKS_FIELD = get_field_from_name(AudioManager, c"_songCharaPlaybacks");
    }
}

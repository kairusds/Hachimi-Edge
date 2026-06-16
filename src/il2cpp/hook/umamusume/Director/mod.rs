use crate::{
    core::{gui::IS_LIVE_SCENE, Hachimi},
    il2cpp::{
        ext::StringExt,
        sql,
        symbols::{get_assembly_image, get_method_addr, get_field_from_name, get_class, Array, IList, SingletonLike},
        types::*
    }
};
use std::{ptr::null_mut, sync::atomic::{AtomicBool, Ordering}};

pub mod LiveLoadSettings;

use LiveLoadSettings::{CharacterInfo, RaceInfo};

static IS_LIVE_PAUSED: AtomicBool = AtomicBool::new(false);

pub fn is_live_paused() -> bool {
    IS_LIVE_PAUSED.load(Ordering::Acquire)
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

static mut GET_LIVECURRENTTIME_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_LiveCurrentTime, GET_LIVECURRENTTIME_ADDR, f32, this: *mut Il2CppObject);

static mut GET_LIVETOTALTIME_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_LiveTotalTime, GET_LIVETOTALTIME_ADDR, f32, this: *mut Il2CppObject);

type PauseLiveFn = extern "C" fn(this: *mut Il2CppObject, is_pause: bool);
pub extern "C" fn PauseLive(this: *mut Il2CppObject, is_pause: bool) {
    get_orig_fn!(PauseLive, PauseLiveFn)(this, is_pause);
    IS_LIVE_PAUSED.store(is_pause, Ordering::Release);
}

static mut ISPAUSELIVE_ADDR: usize = 0;
impl_addr_wrapper_fn!(IsPauseLive, ISPAUSELIVE_ADDR, bool, this: *mut Il2CppObject);

static mut GET_LOADSETTINGS_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_LoadSettings, GET_LOADSETTINGS_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_LIVETIMECONTROLLER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_LiveTimeController, GET_LIVETIMECONTROLLER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut REGISTER_DOWNLOAD_EXTRA_RESOURCE_ADDR: usize = 0;
impl_addr_wrapper_fn!(
    RegisterDownloadExtraResource,
    REGISTER_DOWNLOAD_EXTRA_RESOURCE_ADDR,
    (),
    register: *mut Il2CppObject,
    extra_resource_id: i32
);

def_field_value_accessors!(set set__liveCurrentTime, _LIVECURRENTTIME_FIELD, f32);

fn patch_champions_live(this: *mut Il2CppObject) {
    let config = Hachimi::instance().config.load();

    let load_settings = get_LoadSettings(this);
    if load_settings.is_null() { return; }

    let music_id = LiveLoadSettings::get_MusicId(load_settings);
    if music_id != 1054 { return; }

    let race_info = LiveLoadSettings::get_raceInfo(load_settings);
    if race_info.is_null() { return; }

    let cm_res_id = RaceInfo::get_ChampionsMeetingResourceId(race_info);
    if cm_res_id != 0 { return; }

    RaceInfo::set_ChampionsMeetingResourceId(race_info, config.champions_live_resource_id);
    RaceInfo::set_DateYear(race_info, config.champions_live_year);

    let mscorlib = match get_assembly_image(c"mscorlib.dll") {
        Ok(img) => img,
        Err(_) => return
    };
    let string_class = match get_class(mscorlib, c"System", c"String") {
        Ok(c) => c,
        Err(_) => return
    };
    let chara_name_array = Array::<*mut Il2CppString>::new(string_class, 9);
    let trainer_name_array = Array::<*mut Il2CppString>::new(string_class, 9);
    if chara_name_array.this.is_null() || trainer_name_array.this.is_null() { return; }

    let chara_info_list = LiveLoadSettings::get_CharacterInfoList(load_settings);

    if let Some(ilist) = IList::<*mut Il2CppObject>::new(chara_info_list) {
        for i in 0..9 {
            let mut chara_name = "".to_il2cpp_string();
            let trainer_name = "".to_il2cpp_string();

            if let Some(info) = ilist.get(i as i32) {
                let chara_id = CharacterInfo::get_CharaId(info);
                let mob_id = CharacterInfo::get_MobId(info);

                let name_str = if chara_id == 1 {
                    sql::get_master_text(59, mob_id).unwrap_or_else(|| "???".to_string())
                } else {
                    Hachimi::instance().chara_data.load().get_name(chara_id)
                };
                chara_name = name_str.to_il2cpp_string();
            }

            unsafe {
                chara_name_array.as_slice()[i as usize] = chara_name;
                trainer_name_array.as_slice()[i as usize] = trainer_name;
            }
        }
    }

    RaceInfo::set_CharacterNameArray(race_info, chara_name_array.this);
    RaceInfo::set_TrainerNameArray(race_info, trainer_name_array.this);
    RaceInfo::set_CharacterNameArrayForChampionsText(race_info, null_mut());
    RaceInfo::set_TrainerNameArrayForChampionsText(race_info, null_mut());
}

type AwakeFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Awake(this: *mut Il2CppObject) {
    IS_LIVE_SCENE.store(true, Ordering::Release);
    get_orig_fn!(Awake, AwakeFn)(this);

    IS_LIVE_PAUSED.store(IsPauseLive(this), Ordering::Release);

    if Hachimi::instance().config.load().champions_live_show_text {
        patch_champions_live(this);
    }
}

#[cfg(target_os = "windows")]
type SetupOrientationFn = extern "C" fn(this: *mut Il2CppObject, target_display_mode: i32);

#[cfg(target_os = "windows")]
extern "C" fn SetupOrientation(this: *mut Il2CppObject, mut target_display_mode: i32) {
    const LANDSCAPE_DISPLAY_MODE: i32 = 1;
    const PORTRAIT_DISPLAY_MODE: i32 = 2;

    let config = Hachimi::instance().config.load();
    if config.windows.freeform_window {
        if let Some((width, height)) = crate::windows::wnd_hook::get_client_size() {
            target_display_mode = if width > height {
                LANDSCAPE_DISPLAY_MODE
            } else {
                PORTRAIT_DISPLAY_MODE
            };
        }
    }

    get_orig_fn!(SetupOrientation, SetupOrientationFn)(this, target_display_mode);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live", Director);

    LiveLoadSettings::init(Director);

    unsafe {
        CLASS = Director;
        GET_LIVECURRENTTIME_ADDR = get_method_addr(Director, c"get_LiveCurrentTime", 0);
        GET_LIVETOTALTIME_ADDR = get_method_addr(Director, c"get_LiveTotalTime", 0);
        ISPAUSELIVE_ADDR = get_method_addr(Director, c"IsPauseLive", 0);
        GET_LOADSETTINGS_ADDR = get_method_addr(Director, c"get_LoadSettings", 0);
        GET_LIVETIMECONTROLLER_ADDR = get_method_addr(Director, c"get_LiveTimeController", 0);
        REGISTER_DOWNLOAD_EXTRA_RESOURCE_ADDR = get_method_addr(Director, c"RegisterDownloadExtraResource", 2);
        _LIVECURRENTTIME_FIELD = get_field_from_name(Director, c"_liveCurrentTime");
    }

    let awake_addr = get_method_addr(Director, c"Awake", 0);
    new_hook!(awake_addr, Awake);

    let pause_live_addr = get_method_addr(Director, c"PauseLive", 1);
    new_hook!(pause_live_addr, PauseLive);

    #[cfg(target_os = "windows")]
    {
        let setup_orientation_addr = get_method_addr(Director, c"SetupOrientation", 1);
        new_hook!(setup_orientation_addr, SetupOrientation);
    }
}

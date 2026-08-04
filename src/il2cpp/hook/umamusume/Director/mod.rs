use crate::{
    core::{game::Region, gui::IS_LIVE_SCENE, Hachimi},
    il2cpp::{
        ext::StringExt,
        hook::UnityEngine_CoreModule::Camera,
        sql,
        symbols::{
            get_assembly_image, get_class, get_field_from_name,
            get_method_addr, Array, IList, SingletonLike
        },
        types::*,
    },
};
use std::{
    ptr::null_mut,
    sync::atomic::{AtomicBool, Ordering},
};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "android")]
use crate::il2cpp::hook::umamusume::Screen;

pub mod LiveLoadSettings;

use LiveLoadSettings::{CharacterInfo, RaceInfo};

#[cfg(target_os = "windows")]
use super::{
    CharacterObject, LiveModelController, ModelController,
};
#[cfg(target_os = "windows")]
use crate::{
    windows::free_camera::{self, CameraScene},
    il2cpp::{
        hook::UnityEngine_CoreModule::{GameObject, Transform},
    }
};

#[cfg(target_os = "windows")]
static LIVE_DISABLED_HEADS: free_camera::DisabledHeadStore = once_cell::sync::Lazy::new(free_camera::new_disabled_head_store);

#[cfg(target_os = "windows")]
def_field_object_accessors!(get__multiCameraFinalCompositeArray, set__multiCameraFinalCompositeArray, MULTI_CAMERA_FINAL_COMPOSITE_ARRAY_FIELD, Il2CppArray);
// MCFC = MultiCameraFinalComposite
#[cfg(target_os = "windows")]
static mut SET_MULTI_CAMERA_FADE_VALUE_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(MCFC_set_fadeValue, SET_MULTI_CAMERA_FADE_VALUE_ADDR, (), this: *mut Il2CppObject, value: f32);

#[cfg(target_os = "windows")]
const LIVE_CHARACTER_POSITIONS: &[i32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20, 21,
];

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

#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[repr(i32)]
pub enum DisplayMode {
    None = 0,
    Landscape = 1,
    #[default] Portrait = 2
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

static mut GET_MAIN_CAMERA_OBJECT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_MainCameraObject, GET_MAIN_CAMERA_OBJECT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

#[cfg(target_os = "windows")]
static mut GET_MAIN_CAMERA_TRANSFORM_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(get_MainCameraTransform, GET_MAIN_CAMERA_TRANSFORM_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut SET_DISPLAYMODE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_displayMode, SET_DISPLAYMODE_ADDR, (), this: *mut Il2CppObject, value: DisplayMode);

static mut GETPLAYSONGID_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetPlaySongId, GETPLAYSONGID_ADDR, i32, this: *mut Il2CppObject);

static mut REGISTER_DOWNLOAD_EXTRA_RESOURCE_ADDR: usize = 0;
impl_addr_wrapper_fn!(
    RegisterDownloadExtraResource,
    REGISTER_DOWNLOAD_EXTRA_RESOURCE_ADDR,
    (),
    register: *mut Il2CppObject,
    extra_resource_id: i32
);

#[cfg(target_os = "windows")]
static mut GET_CHARACTER_OBJECT_FROM_POSITION_ID_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(
    GetCharacterObjectFromPositionId, GET_CHARACTER_OBJECT_FROM_POSITION_ID_ADDR,
    *mut Il2CppObject, this: *mut Il2CppObject, id: i32
);

def_field_value_accessors!(set set__liveCurrentTime, _LIVECURRENTTIME_FIELD, f32);
def_field_value_accessors!(get__trainerCameraFovRate, set__trainerCameraFovRate, _TRAINERCAMERAFOVRATE_FIELD, f32);
def_field_value_accessors!(get__trainerCameraFovRateStart, set__trainerCameraFovRateStart, _TRAINERCAMERAFOVRATESTART_FIELD, f32);
def_field_object_accessors!(get__trainerCameraTargetCamera, set__trainerCameraTargetCamera, _TRAINERCAMERATARGETCAMERA_FIELD, Il2CppObject);
def_field_object_accessors!(
    get__trainerCameraTargetCameraTransform, set__trainerCameraTargetCameraTransform,
    _TRAINERCAMERATARGETCAMERATRANSFORM_FIELD, Il2CppObject
);

#[cfg(target_os = "windows")]
pub fn restore_live_disabled_heads(current_index: i32, force_all: bool) {
    free_camera::restore_disabled_heads(&LIVE_DISABLED_HEADS, current_index, force_all);
}

#[cfg(target_os = "windows")]
fn for_each_live_model_controller(
    chara_object: *mut Il2CppObject,
    mut callback: impl FnMut(*mut Il2CppObject),
) {
    let model_array = CharacterObject::get_LiveModelControllerArray(chara_object);
    if model_array.is_null() {
        return;
    }

    let model_array = Array::<*mut Il2CppObject>::from(model_array as *mut Il2CppArray);
    for model_controller in unsafe { model_array.as_slice() }.iter().copied() {
        if !model_controller.is_null() {
            callback(model_controller);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn apply_live_character_options_to_character(chara_object: *mut Il2CppObject) {
    if chara_object.is_null() {
        return;
    }

    let force_visible = free_camera::should_force_live_characters_visible();
    if !force_visible {
        return;
    }

    if force_visible {
        CharacterObject::set_liveCharaVisible(chara_object, true);
        CharacterObject::ApplyVisible(chara_object);
    }

    for_each_live_model_controller(chara_object, |model_controller| {
        ModelController::SetVisible(model_controller, true, true);
        LiveModelController::SetMeshActive(model_controller, true);
    });
}

#[cfg(target_os = "windows")]
pub fn apply_live_character_options_to_list(character_object_list: *mut Il2CppObject) {
    let Some(character_object_list) = IList::<*mut Il2CppObject>::new(character_object_list) else {
        return;
    };

    for chara_object in &character_object_list {
        apply_live_character_options_to_character(chara_object);
    }
}

#[cfg(target_os = "windows")]
pub fn apply_live_character_options(this: *mut Il2CppObject) {
    if this.is_null() {
        return;
    }

    let force_visible = free_camera::should_force_live_characters_visible();
    if !force_visible {
        return;
    }

    for &index in LIVE_CHARACTER_POSITIONS {
        let chara_object = GetCharacterObjectFromPositionId(this, index);
        apply_live_character_options_to_character(chara_object);
    }
}

fn patch_champions_live(this: *mut Il2CppObject) {
    let config = Hachimi::instance().config.load();

    let load_settings = get_LoadSettings(this);
    if load_settings.is_null() {
        return;
    }

    let music_id = GetPlaySongId(this);
    if music_id != 1054 { // Ms. VICTORIA
        return;
    }

    let race_info = LiveLoadSettings::get_raceInfo(load_settings);
    if race_info.is_null() {
        return;
    }

    let cm_res_id = RaceInfo::get_ChampionsMeetingResourceId(race_info);
    if cm_res_id != 0 {
        return;
    }

    RaceInfo::set_ChampionsMeetingResourceId(race_info, config.champions_live_resource_id);
    RaceInfo::set_DateYear(race_info, config.champions_live_year);

    let mscorlib = match get_assembly_image(c"mscorlib.dll") {
        Ok(img) => img,
        Err(_) => return,
    };
    let string_class = match get_class(mscorlib, c"System", c"String") {
        Ok(c) => c,
        Err(_) => return,
    };
    let chara_name_array = Array::<*mut Il2CppString>::new(string_class, 9);
    let trainer_name_array = Array::<*mut Il2CppString>::new(string_class, 9);
    if chara_name_array.this.is_null() || trainer_name_array.this.is_null() {
        return;
    }

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

#[cfg(target_os = "windows")]
fn update_free_camera_live_availability(director: *mut Il2CppObject) {
    free_camera::set_live_music_id(GetPlaySongId(director));
}

#[cfg(target_os = "windows")]
fn force_free_camera_fullscreen(this: *mut Il2CppObject) {
    if this.is_null() || !free_camera::is_scene_enabled(CameraScene::Live) {
        return;
    }

    let composites = get__multiCameraFinalCompositeArray(this);
    if composites.is_null() {
        return;
    }

    let composites = Array::<*mut Il2CppObject>::from(composites);
    for composite in unsafe { composites.as_slice() }.iter().copied() {
        if !composite.is_null() {
            MCFC_set_fadeValue(composite, 0.0);
        }
    }
}

#[cfg(target_os = "windows")]
fn update_live_free_camera_target(this: *mut Il2CppObject) {
    let first_person = free_camera::is_live_first_person();
    let selfie_stick = free_camera::is_live_selfie_stick();
    let head_selfie = selfie_stick && free_camera::is_live_head_selfie();
    if !first_person && !selfie_stick {
        restore_live_disabled_heads(0, true);
        return;
    }

    let mut index = free_camera::live_character_position_index();
    let mut chara_object = GetCharacterObjectFromPositionId(this, index);
    if chara_object.is_null() && index > 0 {
        for fallback in (0..index).rev() {
            chara_object = GetCharacterObjectFromPositionId(this, fallback);
            if !chara_object.is_null() {
                index = fallback;
                break;
            }
        }
    }
    if chara_object.is_null() {
        restore_live_disabled_heads(index, true);
        return;
    }

    let model_array = CharacterObject::get_LiveModelControllerArray(chara_object);
    let model_controller = free_camera::first_enumerable_item(model_array);
    if model_controller.is_null() {
        restore_live_disabled_heads(index, true);
        return;
    }

    let head_transform = LiveModelController::get_HeadTransform(model_controller);
    if head_transform.is_null() {
        restore_live_disabled_heads(index, true);
        return;
    }

    let mut pos = Vector3_t::default();
    let mut rot = Quaternion_t::default();
    Transform::get_position_Injected(head_transform, &mut pos);
    Transform::get_rotation_Injected(head_transform, &mut rot);
    let forward = Transform::get_forward(head_transform);

    let mut root_pos = pos;
    let owner = ModelController::get_OwnerObject(model_controller);
    if !owner.is_null() {
        let owner_transform = GameObject::get_transform(owner);
        if !owner_transform.is_null() {
            Transform::get_position_Injected(owner_transform, &mut root_pos);
        }
    }

    if first_person {
        free_camera::update_first_person(CameraScene::Live, pos, rot, Some(forward));
        free_camera::hide_head_parts(&LIVE_DISABLED_HEADS, model_controller, index);
        restore_live_disabled_heads(index, false);
    } else if head_selfie {
        free_camera::update_live_head_follow(pos, rot, Some(forward));
        restore_live_disabled_heads(0, true);
    } else {
        free_camera::update_live_director_follow_target(pos, root_pos, rot, Some(forward));
        restore_live_disabled_heads(0, true);
    }
}

#[cfg(target_os = "windows")]
fn apply_free_camera_to_main_camera(director: *mut Il2CppObject) {
    if !free_camera::is_scene_enabled(CameraScene::Live) {
        return;
    }

    let camera_transform = get_MainCameraTransform(director);
    if camera_transform.is_null() {
        return;
    }

    let mut position = free_camera::camera_pos();
    Transform::set_position_Injected(camera_transform, &mut position);
    if let Some(mut rotation) = free_camera::camera_rotation() {
        Transform::set_rotation_Injected(camera_transform, &mut rotation);
    }
    else {
        let mut look_at = free_camera::camera_look_at();
        let mut world_up = Vector3_t { x: 0.0, y: 1.0, z: 0.0 };
        Transform::Internal_LookAt_Injected(camera_transform, &mut look_at, &mut world_up);
    }

    let camera = get_MainCameraObject(director);
    if !camera.is_null() {
        if let Some(fov) = free_camera::fov_for_scene(CameraScene::Live) {
            Camera::set_fieldOfView(camera, fov);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn enforce_live_free_camera_output(director: *mut Il2CppObject) {
    force_free_camera_fullscreen(director);
    apply_free_camera_to_main_camera(director);
}

#[cfg(target_os = "windows")]
pub fn apply_paused_free_camera() {
    if !is_live_paused() {
        return;
    }

    let director = instance();
    if director.is_null() {
        return;
    }

    free_camera::set_live_active();
    if !free_camera::is_scene_enabled(CameraScene::Live) {
        return;
    }

    update_live_free_camera_target(director);
    free_camera::refresh_paused_live_camera();
    enforce_live_free_camera_output(director);
}

pub fn is_trainer_live() -> bool {
    let director = instance();
    if director.is_null() { return false; }
    let music_id = GetPlaySongId(director);
    // music_id = sqlite3 master.mdb "SELECT \"index\" FROM text_data WHERE text LIKE 'JPSONGNAME' AND id = 16 AND category = 16;"
    // or category "16: {}" on text_data_dict.json from translation repo
    // 1156 = meni shuki
    music_id == 1156
}

type AwakeFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Awake(this: *mut Il2CppObject) {
    get_orig_fn!(Awake, AwakeFn)(this);
    IS_LIVE_SCENE.store(true, Ordering::Release);
    IS_LIVE_PAUSED.store(IsPauseLive(this), Ordering::Release);

    #[cfg(target_os = "windows")]
    update_free_camera_live_availability(this);

    if is_trainer_live() && Hachimi::instance().config.load().trainer_live_landscape {
        set_displayMode(this, DisplayMode::Landscape);
    }

    if Hachimi::instance().config.load().champions_live_show_text {
        patch_champions_live(this);
    }
}

#[cfg(target_os = "android")]
type ApplyTrainerCameraFovFn = extern "C" fn(this: *mut Il2CppObject);
#[cfg(target_os = "android")]
extern "C" fn ApplyTrainerCameraFov(this: *mut Il2CppObject) {
    get_orig_fn!(ApplyTrainerCameraFov, ApplyTrainerCameraFovFn)(this);

    let config = Hachimi::instance().config.load();
    let force_landscape = (!Screen::get_IsVertical() && config.trainer_live_landscape)
        || config.android.force_orientation_mode == ScreenOrientation_LandscapeLeft;

    if force_landscape {
        let trainer_camera = get__trainerCameraTargetCamera(this);
        if trainer_camera.is_null() {
            return;
        }

        let base_fov = 150.0;
        let fov_rate = get__trainerCameraFovRate(this);
        let fov_rate: f32 = if fov_rate == 0.0 { 1.0 } else { fov_rate.clamp(0.1, 1.0) };
        set__trainerCameraFovRate(this, fov_rate);
        Camera::set_fieldOfView(trainer_camera, base_fov * fov_rate);
    }
}

#[cfg(target_os = "windows")]
type AlterUpdateFn = extern "C" fn(this: *mut Il2CppObject, delta_time: f32, is_update_delta_time: bool);
#[cfg(target_os = "windows")]
extern "C" fn AlterUpdate(this: *mut Il2CppObject, delta_time: f32, is_update_delta_time: bool) {
    free_camera::begin_live_director_update();
    get_orig_fn!(AlterUpdate, AlterUpdateFn)(this, delta_time, is_update_delta_time);
    free_camera::set_live_active();
    apply_live_character_options(this);
    update_live_free_camera_target(this);
    enforce_live_free_camera_output(this);
}


#[cfg(target_os = "windows")]
type SetupOrientationFn = extern "C" fn(this: *mut Il2CppObject, display_mode: DisplayMode);
#[cfg(target_os = "windows")]
extern "C" fn SetupOrientation(this: *mut Il2CppObject, display_mode: DisplayMode) {
    let config = Hachimi::instance().config.load();
    let mut target_display_mode = display_mode;
    if config.windows.freeform_window {
        if let Some((width, height)) = crate::windows::wnd_hook::get_client_size() {
            target_display_mode = if width > height {
                DisplayMode::Landscape
            } else {
                DisplayMode::Portrait
            };
        }
    }

    get_orig_fn!(SetupOrientation, SetupOrientationFn)(this, target_display_mode)
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
        GET_MAIN_CAMERA_OBJECT_ADDR = get_method_addr(Director, c"get_MainCameraObject", 0);
        SET_DISPLAYMODE_ADDR = get_method_addr(Director, c"set_displayMode", 1);
        GETPLAYSONGID_ADDR = get_method_addr(Director, c"GetPlaySongId", 0);

        _LIVECURRENTTIME_FIELD = get_field_from_name(Director, c"_liveCurrentTime");
        _TRAINERCAMERAFOVRATE_FIELD = get_field_from_name(Director, c"_trainerCameraFovRate");
        _TRAINERCAMERAFOVRATESTART_FIELD = get_field_from_name(Director, c"_trainerCameraFovRateStart");
        _TRAINERCAMERATARGETCAMERA_FIELD = get_field_from_name(Director, c"_trainerCameraTargetCamera");
        _TRAINERCAMERATARGETCAMERATRANSFORM_FIELD = get_field_from_name(Director, c"_trainerCameraTargetCameraTransform");

        #[cfg(target_os = "windows")]
        {
            GET_MAIN_CAMERA_TRANSFORM_ADDR = get_method_addr(Director, c"get_MainCameraTransform", 0);
            GET_CHARACTER_OBJECT_FROM_POSITION_ID_ADDR = get_method_addr(Director, c"GetCharacterObjectFromPositionId", 1);
            MULTI_CAMERA_FINAL_COMPOSITE_ARRAY_FIELD = get_field_from_name(Director, c"_multiCameraFinalCompositeArray");

            get_class_or_return!(umamusume, "Gallop.Live", MultiCameraFinalComposite);
            SET_MULTI_CAMERA_FADE_VALUE_ADDR = get_method_addr(MultiCameraFinalComposite, c"set_fadeValue", 1);
        }
    }

    let awake_addr = get_method_addr(Director, c"Awake", 0);
    new_hook!(awake_addr, Awake);

    let pause_live_addr = if Hachimi::instance().game.region == Region::Global {
        get_method_addr(Director, c"PauseModel", 1)
    } else {
        get_method_addr(Director, c"PauseLive", 1)
    };
    
    new_hook!(pause_live_addr, PauseLive);

    #[cfg(target_os = "android")]
    {
        let ApplyTrainerCameraFov_addr = get_method_addr(Director, c"ApplyTrainerCameraFov", 0);
        new_hook!(ApplyTrainerCameraFov_addr, ApplyTrainerCameraFov);
    }

    #[cfg(target_os = "windows")]
    {
        let setup_orientation_addr = get_method_addr(Director, c"SetupOrientation", 1);
        new_hook!(setup_orientation_addr, SetupOrientation);

        let AlterUpdate_addr = get_method_addr(Director, c"AlterUpdate", 2);
        new_hook!(AlterUpdate_addr, AlterUpdate);
    }
}

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    core::Hachimi,
    windows::free_camera::{self, CameraScene},
    il2cpp::{
        hook::UnityEngine_CoreModule::Camera,
        symbols::get_method_addr,
        types::*,
    },
};

use super::{Director, LiveTimelineWorkSheet, LiveTimelineKeyPostFilmDataList};

static LIVE_TIMELINE_CONTROL: AtomicUsize = AtomicUsize::new(0);


#[repr(C)]
#[derive(Default)]
#[allow(dead_code)]
pub struct Vector4_t {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
#[allow(dead_code)]
pub struct PostFilmUpdateInfo {
    pub filmMode: i32,
    pub colorType: i32,
    pub filmPower: f32,
    pub filmOffsetParam: Vector2_t,
    pub filmOptionParam: Vector4_t,
    pub color0: Color_t,
    pub color1: Color_t,
    pub color2: Color_t,
    pub color3: Color_t,
    pub depthPower: f32,
    pub DepthClip: f32,
    pub layerMode: i32,
    pub colorBlend: i32,
    pub inverseVignette: bool,
    pub colorBlendFactor: f32,
    pub movieResId: i32,
    pub movieFrameOffset: i32,
    pub movieTime: f32,
    pub movieReverse: bool,
    pub RollAngle: f32,
    pub FilmScale: Vector2_t,
}

impl Default for PostFilmUpdateInfo {
    fn default() -> Self {
        Self {
            filmMode: 0,
            colorType: 0,
            filmPower: 0.0,
            filmOffsetParam: Default::default(),
            filmOptionParam: Default::default(),
            color0: Color_t {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            color1: Color_t {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            color2: Color_t {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            color3: Color_t {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            depthPower: 0.0,
            DepthClip: 0.0,
            layerMode: 0,
            colorBlend: 0,
            inverseVignette: false,
            colorBlendFactor: 0.0,
            movieResId: 0,
            movieFrameOffset: 0,
            movieTime: 0.0,
            movieReverse: false,
            RollAngle: 0.0,
            FilmScale: Default::default(),
        }
    }
}

#[repr(C)]
#[allow(dead_code)]
struct PostEffectUpdateInfo_DOF {
    pub IsEnableDOF: bool,
    pub forcalSize: f32,
    pub blurSpread: f32,
    pub forcalPosition: Vector3_t,
    pub dofQuality: i32,
    pub dofBlurType: i32,
    pub dofForegroundSize: f32,
    pub dofFocalPoint: f32,
    pub dofSoomthness: f32,
    pub isUseFocalPoint: bool,
    pub BallBlurCurveFactor: f32,
    pub BallBlurBrightnessThreshhold: f32,
    pub BallBlurBrightnessIntensity: f32,
    pub BallBlurSpread: f32,
    pub IsPointBallBlur: bool,
}

fn clear_live_screen_effects(sheet: *mut Il2CppObject) {
    if sheet.is_null() || !free_camera::should_remove_live_screen_effects() {
        return;
    }    

    let post_film_keys = LiveTimelineWorkSheet::get_postFilmKeys(sheet);
    if !post_film_keys.is_null() {
        LiveTimelineKeyPostFilmDataList::Clear(post_film_keys);
    }

    let post_film2_keys = LiveTimelineWorkSheet::get_postFilm2Keys(sheet);
    if !post_film2_keys.is_null() {
        LiveTimelineKeyPostFilmDataList::Clear(post_film2_keys);
    }

    let post_film3_keys = LiveTimelineWorkSheet::get_postFilm3Keys(sheet);
    if !post_film3_keys.is_null() {
        LiveTimelineKeyPostFilmDataList::Clear(post_film3_keys);
    }
}

pub fn set_current(this: *mut Il2CppObject) {
    if !this.is_null() {
        LIVE_TIMELINE_CONTROL.store(this as usize, Ordering::Relaxed);
    }
}

fn clear_current() {
    LIVE_TIMELINE_CONTROL.store(0, Ordering::Relaxed);
}

fn should_remove_live_camera_effects() -> bool {
    free_camera::set_live_active();
    free_camera::should_remove_camera_effects()
}

fn should_override_live_camera() -> bool {
    free_camera::set_live_active();
    free_camera::is_scene_enabled(CameraScene::Live)
}

fn apply_current_live_character_options() {
    let director = Director::instance();
    if !director.is_null() {
        Director::apply_live_character_options(director);
    }
}

type NoArgsFn = extern "C" fn(this: *mut Il2CppObject);

type LiveVoidFrameFn = extern "C" fn(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32);
type LiveBoolFrameFn = extern "C" fn(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) -> bool;
type LiveVoidFrameTimeFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
);

type AlterUpdate_CameraPosFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    sheet_index: i32,
    is_use_camera_motion: bool,
);
extern "C" fn AlterUpdate_CameraPos(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    sheet_index: i32,
    mut is_use_camera_motion: bool,
) {
    free_camera::set_live_active();
    clear_live_screen_effects(sheet);
    let free_camera_active = free_camera::is_scene_enabled(CameraScene::Live);
    let frame = if free_camera_active {
        is_use_camera_motion = false;
        0
    } else {
        current_frame
    };
    get_orig_fn!(AlterUpdate_CameraPos, AlterUpdate_CameraPosFn)(
        this,
        sheet,
        frame,
        current_time,
        sheet_index,
        is_use_camera_motion,
    );
}

type AlterUpdate_CameraLookAtFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    out_look_at: *mut Vector3_t,
);
extern "C" fn AlterUpdate_CameraLookAt(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    out_look_at: *mut Vector3_t,
) {
    free_camera::set_live_active();
    clear_live_screen_effects(sheet);
    get_orig_fn!(AlterUpdate_CameraLookAt, AlterUpdate_CameraLookAtFn)(
        this,
        sheet,
        current_frame,
        current_time,
        out_look_at,
    );

    set_current(this);
    if free_camera::is_scene_enabled(CameraScene::Live) && !out_look_at.is_null() {
        unsafe {
            *out_look_at = free_camera::camera_look_at();
        }
    }
}

extern "C" fn LiveTimelineControl_AlterLateUpdate(this: *mut Il2CppObject) {
    free_camera::set_live_active();
    free_camera::tick();
    get_orig_fn!(LiveTimelineControl_AlterLateUpdate, NoArgsFn)(this);
    apply_current_live_character_options();
    let director = Director::instance();
    if !director.is_null() {
        Director::enforce_live_free_camera_output(director);
    }
}

extern "C" fn LiveTimelineControl_OnDestroy(this: *mut Il2CppObject) {
    Director::restore_live_disabled_heads(0, true);
    clear_current();
    free_camera::end_scene(CameraScene::Live);
    get_orig_fn!(LiveTimelineControl_OnDestroy, NoArgsFn)(this);
}

extern "C" fn AlterUpdate_RadialBlur(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) {
    if !should_remove_live_camera_effects() {
        get_orig_fn!(AlterUpdate_RadialBlur, LiveVoidFrameFn)(this, sheet, current_frame);
    }
}

type SetupPostFilmUpdateDataInfoFn = extern "C" fn(
    this: *mut Il2CppObject,
    updateInfo: *mut PostFilmUpdateInfo,
    curData: *mut Il2CppObject,
    nextData: *mut Il2CppObject,
    currentFrame: i32,
);
extern "C" fn SetupPostFilmUpdateDataInfo(
    this: *mut Il2CppObject,
    updateInfo: *mut PostFilmUpdateInfo,
    curData: *mut Il2CppObject,
    nextData: *mut Il2CppObject,
    currentFrame: i32,
) {
    get_orig_fn!(SetupPostFilmUpdateDataInfo, SetupPostFilmUpdateDataInfoFn)(
        this, updateInfo, curData, nextData, currentFrame,
    );

    if should_remove_live_camera_effects() {
        unsafe { *updateInfo = PostFilmUpdateInfo::default(); }
    }
}

type SetupDOFUpdateInfoFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
    camera_look_at: Vector3_t,
);
extern "C" fn SetupDOFUpdateInfo(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
    camera_look_at: Vector3_t,
) {
    get_orig_fn!(SetupDOFUpdateInfo, SetupDOFUpdateInfoFn)(
        this,
        update_info,
        cur_data,
        next_data,
        current_frame,
        camera_look_at,
    );

    if should_remove_live_camera_effects() {
        unsafe {
            (*update_info).IsEnableDOF = false;
            (*update_info).isUseFocalPoint = false;
            (*update_info).IsPointBallBlur = false;
        }
    }
}

type SetupRadialBlurInfoFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
);
extern "C" fn SetupRadialBlurInfo(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
) {
    if should_remove_live_camera_effects() {
        return;
    }
    get_orig_fn!(SetupRadialBlurInfo, SetupRadialBlurInfoFn)(
        this,
        update_info,
        cur_data,
        next_data,
        current_frame,
    );
}

macro_rules! live_skip_void_frame {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) {
            if should_remove_live_camera_effects() {
                return;
            }
            get_orig_fn!($hook, $type)(this, sheet, current_frame);
        }
    };
}

macro_rules! live_secondary_camera_void_frame {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) {
            let _guard = free_camera::begin_live_secondary_camera_update();
            get_orig_fn!($hook, $type)(this, sheet, current_frame);
        }
    };
}

macro_rules! live_main_camera_void_frame {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) {
            if should_override_live_camera() {
                return;
            }
            get_orig_fn!($hook, $type)(this, sheet, current_frame);
        }
    };
}

macro_rules! live_secondary_camera_void_frame_time {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(
            this: *mut Il2CppObject,
            sheet: *mut Il2CppObject,
            current_frame: i32,
            current_time: f32,
        ) {
            let _guard = free_camera::begin_live_secondary_camera_update();
            get_orig_fn!($hook, $type)(this, sheet, current_frame, current_time);
        }
    };
}

live_secondary_camera_void_frame_time!(AlterUpdate_MultiCameraPosition, LiveVoidFrameTimeFn);
live_secondary_camera_void_frame_time!(AlterUpdate_MultiCameraLookAt, LiveVoidFrameTimeFn);
live_secondary_camera_void_frame!(AlterUpdate_MultiCameraRadialBlur, LiveVoidFrameFn);
live_secondary_camera_void_frame_time!(AlterUpdate_EyeCameraPosition, LiveVoidFrameTimeFn);
live_secondary_camera_void_frame_time!(AlterUpdate_MonitorCameraPosition, LiveVoidFrameTimeFn);
live_skip_void_frame!(AlterUpdate_PostEffect_BloomDiffusion, LiveVoidFrameFn);
live_skip_void_frame!(AlterUpdate_TiltShift, LiveVoidFrameFn);
live_main_camera_void_frame!(AlterUpdate_CameraLayer, LiveVoidFrameFn);
live_main_camera_void_frame!(AlterUpdate_CameraSwitcher, LiveVoidFrameFn);
live_main_camera_void_frame!(AlterUpdate_CameraMotion, LiveVoidFrameFn);
live_main_camera_void_frame!(AlterUpdate_HandShakeCamera, LiveVoidFrameFn);
live_secondary_camera_void_frame_time!(AlterUpdate_MonitorCameraLookAt, LiveVoidFrameTimeFn);
live_secondary_camera_void_frame_time!(AlterUpdate_EyeCameraLookAt, LiveVoidFrameTimeFn);

extern "C" fn AlterLateUpdate_CameraMotion(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) -> bool {
    if should_override_live_camera() {
        return false;
    }
    get_orig_fn!(AlterLateUpdate_CameraMotion, LiveBoolFrameFn)(this, sheet, current_frame)
}

extern "C" fn AlterUpdate_CameraFov(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) {
    if should_override_live_camera() {
        return;
    }

    if Director::is_trainer_live() {
        let config = Hachimi::instance().config.load();
        let director = Director::instance();
        let camera = Director::get_MainCameraObject(director);

        if !camera.is_null() && config.trainer_live_landscape {
            Camera::set_fieldOfView(camera, 150.0);
            return;
        }
    }
    get_orig_fn!(AlterUpdate_CameraFov, LiveVoidFrameFn)(this, sheet, current_frame);
}

extern "C" fn AlterUpdate_CameraRoll(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) {
    if should_override_live_camera() {
        return;
    }
    get_orig_fn!(AlterUpdate_CameraRoll, LiveVoidFrameFn)(this, sheet, current_frame);
}

type LiveFormationOffsetFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    character_object_list: *mut Il2CppObject,
    change_visibility: bool,
);
extern "C" fn AlterUpdate_FormationOffset(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    character_object_list: *mut Il2CppObject,
    mut change_visibility: bool,
) {
    free_camera::set_live_active();
    let disable_teleport = free_camera::should_disable_live_character_teleport();
    let frame = if disable_teleport { 0 } else { current_frame };
    if disable_teleport || free_camera::should_force_live_characters_visible() {
        change_visibility = false;
    }

    // Keep the formation-offset timeline at its initial pose when free camera
    // ignores the authored camera motion. This removes camera-directed teleports
    // without forcing character transform nodes to a shared local position.
    get_orig_fn!(AlterUpdate_FormationOffset, LiveFormationOffsetFn)(
        this,
        sheet,
        frame,
        character_object_list,
        change_visibility,
    );

    Director::apply_live_character_options_to_list(character_object_list);
    apply_current_live_character_options();
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live.Cutt", LiveTimelineControl);

    let AlterUpdate_CameraPos_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_CameraPos", 5);
    new_hook!(AlterUpdate_CameraPos_addr, AlterUpdate_CameraPos);

    let AlterUpdate_CameraLookAt_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_CameraLookAt", 4);
    new_hook!(AlterUpdate_CameraLookAt_addr, AlterUpdate_CameraLookAt);

    let LiveTimelineControl_AlterLateUpdate_addr = get_method_addr(LiveTimelineControl, c"AlterLateUpdate", 0);
    new_hook!(LiveTimelineControl_AlterLateUpdate_addr, LiveTimelineControl_AlterLateUpdate);

    let LiveTimelineControl_OnDestroy_addr = get_method_addr(LiveTimelineControl, c"OnDestroy", 0);
    new_hook!(LiveTimelineControl_OnDestroy_addr, LiveTimelineControl_OnDestroy);

    let AlterUpdate_RadialBlur_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_RadialBlur", 2);
    new_hook!(AlterUpdate_RadialBlur_addr, AlterUpdate_RadialBlur);

    let SetupPostFilmUpdateDataInfo_addr = get_method_addr(LiveTimelineControl, c"SetupPostFilmUpdateDataInfo", 4);
    new_hook!(SetupPostFilmUpdateDataInfo_addr, SetupPostFilmUpdateDataInfo);

    let SetupDOFUpdateInfo_addr = get_method_addr(LiveTimelineControl, c"SetupDOFUpdateInfo", 5);
    new_hook!(SetupDOFUpdateInfo_addr, SetupDOFUpdateInfo);

    let SetupRadialBlurInfo_addr = get_method_addr(LiveTimelineControl, c"SetupRadialBlurInfo", 4);
    new_hook!(SetupRadialBlurInfo_addr, SetupRadialBlurInfo);

    let AlterUpdate_MultiCameraRadialBlur_addr = get_method_addr(
        LiveTimelineControl,
        c"AlterUpdate_MultiCameraRadialBlur",
        2,
    );
    new_hook!(AlterUpdate_MultiCameraRadialBlur_addr, AlterUpdate_MultiCameraRadialBlur);

    let AlterUpdate_EyeCameraPosition_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_EyeCameraPosition", 3);
    new_hook!(AlterUpdate_EyeCameraPosition_addr, AlterUpdate_EyeCameraPosition);

    let AlterUpdate_MonitorCameraPosition_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_MonitorCameraPosition", 3);
    new_hook!(AlterUpdate_MonitorCameraPosition_addr, AlterUpdate_MonitorCameraPosition);

    let AlterUpdate_PostEffect_BloomDiffusion_addr = get_method_addr(
        LiveTimelineControl,
        c"AlterUpdate_PostEffect_BloomDiffusion",
        2,
    );
    new_hook!(AlterUpdate_PostEffect_BloomDiffusion_addr, AlterUpdate_PostEffect_BloomDiffusion);

    let AlterUpdate_TiltShift_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_TiltShift", 2);
    new_hook!(AlterUpdate_TiltShift_addr, AlterUpdate_TiltShift);

    let AlterUpdate_CameraLayer_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_CameraLayer", 2);
    new_hook!(AlterUpdate_CameraLayer_addr, AlterUpdate_CameraLayer);

    let AlterUpdate_CameraFov_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_CameraFov", 2);
    new_hook!(AlterUpdate_CameraFov_addr, AlterUpdate_CameraFov);

    let AlterUpdate_CameraRoll_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_CameraRoll", 2);
    new_hook!(AlterUpdate_CameraRoll_addr, AlterUpdate_CameraRoll);

    let AlterUpdate_CameraMotion_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_CameraMotion", 2);
    new_hook!(AlterUpdate_CameraMotion_addr, AlterUpdate_CameraMotion);

    let AlterLateUpdate_CameraMotion_addr = get_method_addr(LiveTimelineControl, c"AlterLateUpdate_CameraMotion", 2);
    new_hook!(AlterLateUpdate_CameraMotion_addr, AlterLateUpdate_CameraMotion);

    let AlterUpdate_HandShakeCamera_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_HandShakeCamera", 2);
    new_hook!(AlterUpdate_HandShakeCamera_addr, AlterUpdate_HandShakeCamera);

    let AlterUpdate_CameraSwitcher_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_CameraSwitcher", 2);
    new_hook!(AlterUpdate_CameraSwitcher_addr, AlterUpdate_CameraSwitcher);

    let AlterUpdate_MonitorCameraLookAt_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_MonitorCameraLookAt", 3);
    new_hook!(AlterUpdate_MonitorCameraLookAt_addr, AlterUpdate_MonitorCameraLookAt);

    let AlterUpdate_EyeCameraLookAt_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_EyeCameraLookAt", 3);
    new_hook!(AlterUpdate_EyeCameraLookAt_addr, AlterUpdate_EyeCameraLookAt);

    let AlterUpdate_MultiCameraPosition_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_MultiCameraPosition", 3);
    new_hook!(AlterUpdate_MultiCameraPosition_addr, AlterUpdate_MultiCameraPosition);

    let AlterUpdate_MultiCameraLookAt_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_MultiCameraLookAt", 3);
    new_hook!(AlterUpdate_MultiCameraLookAt_addr, AlterUpdate_MultiCameraLookAt);

    let AlterUpdate_FormationOffset_addr = get_method_addr(LiveTimelineControl, c"AlterUpdate_FormationOffset", 4);
    new_hook!(AlterUpdate_FormationOffset_addr, AlterUpdate_FormationOffset);
}

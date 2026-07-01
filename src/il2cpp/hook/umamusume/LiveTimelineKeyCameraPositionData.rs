use crate::{
    core::free_camera::{self, CameraScene, FreeCameraMode},
    il2cpp::{
        ext::Il2CppObjectExt,
        symbols::{get_class, get_field_from_name, get_method_addr, set_field_value},
        types::*,
    },
};

static mut LIVE_TIMELINE_KEY_MULTI_CAMERA_POSITION_DATA_CLASS: *mut Il2CppClass = 0 as _;

fn is_multi_camera_position_data(this: *mut Il2CppObject) -> bool {
    if this.is_null() {
        return false;
    }

    let class = unsafe { LIVE_TIMELINE_KEY_MULTI_CAMERA_POSITION_DATA_CLASS };
    !class.is_null() && unsafe { (*this).klass() == class }
}

type GetCameraPosFn = extern "C" fn(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
) -> *mut Vector3_t;
type GetCameraPos2Fn = extern "C" fn(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
    set_type: i32,
) -> *mut Vector3_t;

extern "C" fn GetCameraPos(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
) -> *mut Vector3_t {
    free_camera::set_live_active();

    if is_multi_camera_position_data(this) {
        return get_orig_fn!(GetCameraPos, GetCameraPosFn)(ret, this, timeline_control);
    }

    if free_camera::is_scene_enabled(CameraScene::Live)
        && free_camera::mode() == FreeCameraMode::SelfieStick
    {
        let field = get_field_from_name(unsafe { (*this).klass() }, c"setType");
        if !field.is_null() {
            set_field_value(this, field, &1i32);
        }
    }

    let result = get_orig_fn!(GetCameraPos, GetCameraPosFn)(ret, this, timeline_control);
    if free_camera::is_scene_enabled(CameraScene::Live) && !result.is_null() {
        unsafe {
            *result = free_camera::camera_pos();
        }
    }
    result
}

extern "C" fn GetCameraPos2(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
    set_type: i32,
) -> *mut Vector3_t {
    free_camera::set_live_active();

    if is_multi_camera_position_data(this) {
        return get_orig_fn!(GetCameraPos2, GetCameraPos2Fn)(
            ret,
            this,
            timeline_control,
            set_type,
        );
    }

    let result =
        get_orig_fn!(GetCameraPos2, GetCameraPos2Fn)(ret, this, timeline_control, set_type);
    if free_camera::is_scene_enabled(CameraScene::Live) && !result.is_null() {
        unsafe {
            *result = free_camera::camera_pos();
        }
    }
    result
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(camera_pos_data) = get_class(
        umamusume,
        c"Gallop.Live.Cutt",
        c"LiveTimelineKeyCameraPositionData",
    ) {
        let GetCameraPos_addr = get_method_addr(camera_pos_data, c"GetValue", 1);
        let GetCameraPos2_addr = get_method_addr(camera_pos_data, c"GetValue", 2);
        new_hook!(GetCameraPos_addr, GetCameraPos);
        new_hook!(GetCameraPos2_addr, GetCameraPos2);
    }

    if let Ok(multi_camera_pos_data) = get_class(
        umamusume,
        c"Gallop.Live.Cutt",
        c"LiveTimelineKeyMultiCameraPositionData",
    ) {
        unsafe {
            LIVE_TIMELINE_KEY_MULTI_CAMERA_POSITION_DATA_CLASS = multi_camera_pos_data;
        }
    }
}

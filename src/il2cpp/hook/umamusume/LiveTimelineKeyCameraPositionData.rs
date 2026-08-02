use crate::{
    windows::free_camera::{self, CameraScene, FreeCameraMode},
    il2cpp::{
        ext::Il2CppObjectExt,
        symbols::{get_field_from_name, get_method_addr},
        types::*,
    },
};
use super::LiveTimelineKeyMultiCameraPositionData;


#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LiveCameraPositionType {
    Direct = 0,
    Character = 1
}

def_field_value_accessors!(get_setType, set_setType, SETTYPE_FIELD, LiveCameraPositionType);

fn is_multi_camera_position_data(this: *mut Il2CppObject) -> bool {
    if this.is_null() {
        return false;
    }
    let class = LiveTimelineKeyMultiCameraPositionData::class();
    !class.is_null() && unsafe { (*this).klass() == class }
}

type GetValueFn = extern "C" fn(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
) -> *mut Vector3_t;
extern "C" fn GetValue(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
) -> *mut Vector3_t {
    free_camera::set_live_active();

    if free_camera::is_live_secondary_camera_update() ||
        is_multi_camera_position_data(this)
    {
        return get_orig_fn!(GetValue, GetValueFn)(ret, this, timeline_control);
    }

    if free_camera::is_scene_enabled(CameraScene::Live) && free_camera::mode() == FreeCameraMode::SelfieStick {
        set_setType(this, LiveCameraPositionType::Character);
    }

    let result = get_orig_fn!(GetValue, GetValueFn)(ret, this, timeline_control);
    if free_camera::is_scene_enabled(CameraScene::Live) && !result.is_null() {
        unsafe {
            *result = free_camera::camera_pos();
        }
    }
    result
}

type GetValue2Fn = extern "C" fn(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
    set_type: LiveCameraPositionType,
) -> *mut Vector3_t;
extern "C" fn GetValue2(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
    set_type: LiveCameraPositionType,
) -> *mut Vector3_t {
    free_camera::set_live_active();

    if free_camera::is_live_secondary_camera_update() ||
        is_multi_camera_position_data(this)
    {
        return get_orig_fn!(GetValue2, GetValue2Fn)(
            ret,
            this,
            timeline_control,
            set_type,
        );
    }

    let result = get_orig_fn!(GetValue2, GetValue2Fn)(ret, this, timeline_control, set_type);
    if free_camera::is_scene_enabled(CameraScene::Live) && !result.is_null() {
        unsafe {
            *result = free_camera::camera_pos();
        }
    }
    result
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live.Cutt", LiveTimelineKeyCameraPositionData);

    let GetValue_addr = get_method_addr(LiveTimelineKeyCameraPositionData, c"GetValue", 1);
    new_hook!(GetValue_addr, GetValue);

    let GetValue2_addr = get_method_addr(LiveTimelineKeyCameraPositionData, c"GetValue", 2);
    new_hook!(GetValue2_addr, GetValue2);
    
    unsafe {
        SETTYPE_FIELD = get_field_from_name(LiveTimelineKeyCameraPositionData, c"setType");
    }
}

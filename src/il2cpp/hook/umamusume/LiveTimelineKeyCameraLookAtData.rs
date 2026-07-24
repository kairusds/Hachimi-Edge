use crate::{
    core::free_camera::{self, CameraScene, FreeCameraMode},
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

use super::LiveTimelineControl;

type GetCharacterWorldPosFn = extern "C" fn(
    ret: *mut Vector3_t,
    timeline_control: *mut Il2CppObject,
    pos_flag: i32,
    chara_parts: i32,
    chara_pos: *mut Vector3_t,
    offset: *mut Vector3_t,
    is_attached_to_props: bool,
    props_index: i32,
    props_attach_node_index: i32,
) -> *mut Vector3_t;

extern "C" fn GetCharacterWorldPos(
    ret: *mut Vector3_t,
    timeline_control: *mut Il2CppObject,
    mut pos_flag: i32,
    mut chara_parts: i32,
    chara_pos: *mut Vector3_t,
    offset: *mut Vector3_t,
    mut is_attached_to_props: bool,
    mut props_index: i32,
    mut props_attach_node_index: i32,
) -> *mut Vector3_t {
    free_camera::set_live_active();
    LiveTimelineControl::set_current(timeline_control);

    if free_camera::is_live_secondary_camera_update() {
        return get_orig_fn!(GetCharacterWorldPos, GetCharacterWorldPosFn)(
            ret,
            timeline_control,
            pos_flag,
            chara_parts,
            chara_pos,
            offset,
            is_attached_to_props,
            props_index,
            props_attach_node_index,
        );
    }

    let is_selfie_stick = free_camera::is_scene_enabled(CameraScene::Live)
        && free_camera::mode() == FreeCameraMode::SelfieStick;
    let is_head_selfie = free_camera::is_live_head_selfie();

    if is_selfie_stick {
        pos_flag = free_camera::live_position_flag();
        chara_parts = free_camera::live_part();
        unsafe {
            if !chara_pos.is_null() {
                *chara_pos = Vector3_t::default();
            }
            if !offset.is_null() {
                *offset = Vector3_t::default();
            }
        }
        is_attached_to_props = false;
        props_index = 0;
        props_attach_node_index = 0;
    }

    let result = get_orig_fn!(GetCharacterWorldPos, GetCharacterWorldPosFn)(
        ret,
        timeline_control,
        pos_flag,
        chara_parts,
        chara_pos,
        offset,
        is_attached_to_props,
        props_index,
        props_attach_node_index,
    );

    if is_selfie_stick && !result.is_null() {
        if is_head_selfie {
            free_camera::update_live_head_part_target(unsafe { *result });
        } else {
            free_camera::update_live_follow_position_target(unsafe { *result });
        }
    }
    result
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(camera_lookat_data) = get_class(
        umamusume,
        c"Gallop.Live.Cutt",
        c"LiveTimelineKeyCameraLookAtData",
    ) {
        let GetCharacterWorldPos_addr =
            get_method_addr(camera_lookat_data, c"GetCharacterWorldPos", 8);
        new_hook!(GetCharacterWorldPos_addr, GetCharacterWorldPos);
    }
}

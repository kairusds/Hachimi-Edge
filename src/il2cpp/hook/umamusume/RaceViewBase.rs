use std::ptr::null_mut;

use crate::{
    core::free_camera::{self, CameraScene},
    il2cpp::{
        ext::StringExt,
        hook::UnityEngine_CoreModule::Transform,
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

use super::{RaceModelController, free_camera as free_camera_hooks};

static RACE_DISABLED_HEADS: free_camera_hooks::DisabledHeadStore =
    once_cell::sync::Lazy::new(free_camera_hooks::new_disabled_head_store);

static mut GET_MODEL_CONTROLLER_ADDR: usize = 0;

pub fn GetModelController(this: *mut Il2CppObject, index: i32) -> *mut Il2CppObject {
    if unsafe { GET_MODEL_CONTROLLER_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject, i32) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(GET_MODEL_CONTROLLER_ADDR) };
    func(this, index)
}

pub(crate) fn restore_race_disabled_heads(current_index: i32, force_all: bool) {
    free_camera_hooks::restore_disabled_heads(&RACE_DISABLED_HEADS, current_index, force_all);
}

type NoArgsFn = extern "C" fn(this: *mut Il2CppObject);

extern "C" fn RaceViewBase_LateUpdateView(this: *mut Il2CppObject) {
    let first_person = free_camera::is_race_first_person();
    let head_selfie = free_camera::is_race_head_selfie();
    if first_person || head_selfie {
        let index = free_camera::race_model_index();
        let model_controller = GetModelController(this, index);
        if !model_controller.is_null() {
            let empty = "".to_il2cpp_string();
            let eye_left = RaceModelController::GetPrefabAttachTransform(model_controller, 0x7, empty);
            let eye_right = RaceModelController::GetPrefabAttachTransform(model_controller, 0x8, empty);
            if !eye_left.is_null() && !eye_right.is_null() {
                let mut pos_left = Vector3_t::default();
                let mut pos_right = Vector3_t::default();
                let mut rot_left = Quaternion_t::default();
                let mut rot_right = Quaternion_t::default();

                Transform::get_position_Injected(eye_left, &mut pos_left);
                Transform::get_position_Injected(eye_right, &mut pos_right);
                Transform::get_rotation_Injected(eye_left, &mut rot_left);
                Transform::get_rotation_Injected(eye_right, &mut rot_right);

                let pos = Vector3_t {
                    x: (pos_left.x + pos_right.x) * 0.5,
                    y: (pos_left.y + pos_right.y) * 0.5,
                    z: (pos_left.z + pos_right.z) * 0.5,
                };
                let rot = free_camera::slerp_quaternion(rot_left, rot_right, 0.5);
                if first_person {
                    free_camera::update_first_person(CameraScene::Race, pos, rot, None);
                    free_camera_hooks::hide_head_parts(&RACE_DISABLED_HEADS, model_controller, index);
                    restore_race_disabled_heads(index, false);
                }
                else {
                    free_camera::update_race_head_follow(pos, rot);
                    restore_race_disabled_heads(0, true);
                }
            }
        }
    }
    else {
        restore_race_disabled_heads(0, true);
    }

    get_orig_fn!(RaceViewBase_LateUpdateView, NoArgsFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(race_view_base) = get_class(umamusume, c"Gallop", c"RaceViewBase") {
        unsafe {
            GET_MODEL_CONTROLLER_ADDR =
                get_method_addr(race_view_base, c"GetModelController", 1);
        }
        let RaceViewBase_LateUpdateView_addr = get_method_addr(race_view_base, c"LateUpdateView", 0);
        new_hook!(RaceViewBase_LateUpdateView_addr, RaceViewBase_LateUpdateView);
    }
}

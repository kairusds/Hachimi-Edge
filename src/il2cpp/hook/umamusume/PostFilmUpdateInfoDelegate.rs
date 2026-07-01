use crate::{
    core::free_camera,
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

#[repr(C)]
#[derive(Default)]
struct Vector4_t {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[repr(C)]
struct PostFilmUpdateInfo {
    film_mode: i32,
    color_type: i32,
    film_power: f32,
    film_offset_param: Vector2_t,
    film_option_param: Vector4_t,
    color0: Color_t,
    color1: Color_t,
    color2: Color_t,
    color3: Color_t,
    depth_power: f32,
    depth_clip: f32,
    layer_mode: i32,
    color_blend: i32,
    inverse_vignette: bool,
    color_blend_factor: f32,
    movie_res_id: i32,
    movie_frame_offset: i32,
    movie_time: f32,
    movie_reverse: bool,
    roll_angle: f32,
    film_scale: Vector2_t,
}

impl Default for PostFilmUpdateInfo {
    fn default() -> Self {
        Self {
            film_mode: 0,
            color_type: 0,
            film_power: 0.0,
            film_offset_param: Default::default(),
            film_option_param: Default::default(),
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
            depth_power: 0.0,
            depth_clip: 0.0,
            layer_mode: 0,
            color_blend: 0,
            inverse_vignette: false,
            color_blend_factor: 0.0,
            movie_res_id: 0,
            movie_frame_offset: 0,
            movie_time: 0.0,
            movie_reverse: false,
            roll_angle: 0.0,
            film_scale: Default::default(),
        }
    }
}

type PostFilmUpdateInfoDelegateInvokeFn =
    extern "C" fn(this: *mut Il2CppObject, update_info: *mut PostFilmUpdateInfo);
type MultiCameraPostFilmUpdateInfoDelegateInvokeFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut PostFilmUpdateInfo,
    multi_camera_no: i32,
);

fn disable(update_info: *mut PostFilmUpdateInfo) {
    if let Some(update_info) = unsafe { update_info.as_mut() } {
        *update_info = PostFilmUpdateInfo::default();
    }
}

extern "C" fn PostFilmUpdateInfoDelegate_Invoke(
    this: *mut Il2CppObject,
    update_info: *mut PostFilmUpdateInfo,
) {
    free_camera::set_live_active();
    let remove_camera_effects = free_camera::should_remove_camera_effects();
    if remove_camera_effects {
        disable(update_info);
    }

    get_orig_fn!(
        PostFilmUpdateInfoDelegate_Invoke,
        PostFilmUpdateInfoDelegateInvokeFn
    )(this, update_info);

    if remove_camera_effects {
        disable(update_info);
    }
}

extern "C" fn MultiCameraPostFilmUpdateInfoDelegate_Invoke(
    this: *mut Il2CppObject,
    update_info: *mut PostFilmUpdateInfo,
    multi_camera_no: i32,
) {
    free_camera::set_live_active();
    let remove_camera_effects = free_camera::should_remove_camera_effects();
    if remove_camera_effects {
        disable(update_info);
    }

    get_orig_fn!(
        MultiCameraPostFilmUpdateInfoDelegate_Invoke,
        MultiCameraPostFilmUpdateInfoDelegateInvokeFn
    )(this, update_info, multi_camera_no);

    if remove_camera_effects {
        disable(update_info);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(post_film_update_info_delegate) = get_class(
        umamusume,
        c"Gallop.Live.Cutt",
        c"PostFilmUpdateInfoDelegate",
    ) {
        let PostFilmUpdateInfoDelegate_Invoke_addr =
            get_method_addr(post_film_update_info_delegate, c"Invoke", 1);
        new_hook!(
            PostFilmUpdateInfoDelegate_Invoke_addr,
            PostFilmUpdateInfoDelegate_Invoke
        );
    }

    if let Ok(multi_camera_post_film_update_info_delegate) = get_class(
        umamusume,
        c"Gallop.Live.Cutt",
        c"MultiCameraPostFilmUpdateInfoDelegate",
    ) {
        let MultiCameraPostFilmUpdateInfoDelegate_Invoke_addr =
            get_method_addr(multi_camera_post_film_update_info_delegate, c"Invoke", 2);
        new_hook!(
            MultiCameraPostFilmUpdateInfoDelegate_Invoke_addr,
            MultiCameraPostFilmUpdateInfoDelegate_Invoke
        );
    }
}

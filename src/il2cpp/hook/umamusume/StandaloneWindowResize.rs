use std::{ptr::null_mut, sync::atomic::{AtomicU8, Ordering}};

use crate::{
    core::Hachimi,
    il2cpp::{
        symbols::{get_field_from_name, get_method_addr, set_static_field_value},
        types::*,
    },
};

static mut WINDOW_LAST_WIDTH_FIELD: *mut FieldInfo = null_mut();
static mut WINDOW_LAST_HEIGHT_FIELD: *mut FieldInfo = null_mut();
static mut ASPECT_RATIO_FIELD: *mut FieldInfo = null_mut();
static mut IS_PREVENT_RESHAPE_FIELD: *mut FieldInfo = null_mut();
static mut IS_VIRT_FIELD: *mut FieldInfo = null_mut();
static mut IS_WINDOW_SIZE_CHANGING_FIELD: *mut FieldInfo = null_mut();
static mut IS_WINDOW_DRAGGING_FIELD: *mut FieldInfo = null_mut();

static mut SAVE_CHANGED_WIDTH_ADDR: usize = 0;
impl_addr_wrapper_fn!(SaveChangedWidth, SAVE_CHANGED_WIDTH_ADDR, (), width: f32, height: f32);

static mut ENABLE_WINDOW_HIT_TEST_ADDR: usize = 0;
impl_addr_wrapper_fn!(EnableWindowHitTest, ENABLE_WINDOW_HIT_TEST_ADDR, (),);

static GET_LIMIT_SIZE_HOOK_ID: AtomicU8 = AtomicU8::new(1);
static DISABLE_MAXIMIZEBOX_HOOK_ID: AtomicU8 = AtomicU8::new(2);
static RESHAPE_ASPECT_RATIO_HOOK_ID: AtomicU8 = AtomicU8::new(3);
static KEEP_ASPECT_RATIO_HOOK_ID: AtomicU8 = AtomicU8::new(4);

#[inline(always)]
fn preserve_hook_identity(identity: &AtomicU8) {
    std::hint::black_box(identity.load(Ordering::Relaxed));
}

fn freeform_enabled() -> bool {
    Hachimi::instance().config.load().windows.freeform_window
}

type GetLimitSizeFn = extern "C" fn() -> Vector2_t;
extern "C" fn GetLimitSize() -> Vector2_t {
    preserve_hook_identity(&GET_LIMIT_SIZE_HOOK_ID);
    if freeform_enabled() {
        return Vector2_t { x: f32::MAX, y: f32::MAX };
    }

    get_orig_fn!(GetLimitSize, GetLimitSizeFn)()
}

type NoArgsFn = extern "C" fn();
extern "C" fn DisableMaximizebox() {
    preserve_hook_identity(&DISABLE_MAXIMIZEBOX_HOOK_ID);
    if freeform_enabled() {
        crate::windows::wnd_hook::apply_freeform_window_style();
        return;
    }

    get_orig_fn!(DisableMaximizebox, NoArgsFn)();
}

type ResizeFn = extern "C" fn(width: f32, height: f32);
extern "C" fn ReshapeAspectRatio(width: f32, height: f32) {
    preserve_hook_identity(&RESHAPE_ASPECT_RATIO_HOOK_ID);
    if !freeform_enabled() {
        get_orig_fn!(ReshapeAspectRatio, ResizeFn)(width, height);
    }
}

extern "C" fn KeepAspectRatio(width: f32, height: f32) {
    preserve_hook_identity(&KEEP_ASPECT_RATIO_HOOK_ID);
    if freeform_enabled() {
        crate::windows::wnd_hook::apply_freeform_window_style();
        return;
    }

    get_orig_fn!(KeepAspectRatio, ResizeFn)(width, height);
}

pub fn update_window_state(client_width: i32, client_height: i32, window_width: i32, window_height: i32) {
    unsafe {
        set_static_field_value(WINDOW_LAST_WIDTH_FIELD, window_width);
        set_static_field_value(WINDOW_LAST_HEIGHT_FIELD, window_height);
        set_static_field_value(ASPECT_RATIO_FIELD, client_width as f32 / client_height as f32);
        set_static_field_value(IS_PREVENT_RESHAPE_FIELD, true);
        set_static_field_value(IS_VIRT_FIELD, client_width < client_height);
    }
    SaveChangedWidth(window_width as f32, window_height as f32);
}

pub fn finish_window_update() {
    unsafe {
        set_static_field_value(IS_PREVENT_RESHAPE_FIELD, false);
    }
    EnableWindowHitTest();
}

pub fn set_is_prevent_reshape(value: bool) {
    unsafe {
        set_static_field_value(IS_PREVENT_RESHAPE_FIELD, value);
    }
}

pub fn set_is_window_size_changing(value: bool) {
    unsafe {
        set_static_field_value(IS_WINDOW_SIZE_CHANGING_FIELD, value);
    }
}

pub fn set_is_window_dragging(value: bool) {
    unsafe {
        set_static_field_value(IS_WINDOW_DRAGGING_FIELD, value);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StandaloneWindowResize);

    let GetLimitSize_addr = get_method_addr(StandaloneWindowResize, c"GetLimitSize", 0);
    let DisableMaximizebox_addr = get_method_addr(StandaloneWindowResize, c"DisableMaximizebox", 0);
    let ReshapeAspectRatio_addr = get_method_addr(StandaloneWindowResize, c"ReshapeAspectRatio", 2);
    let KeepAspectRatio_addr = get_method_addr(StandaloneWindowResize, c"KeepAspectRatio", 2);

    new_hook!(GetLimitSize_addr, GetLimitSize);
    new_hook!(DisableMaximizebox_addr, DisableMaximizebox);
    new_hook!(ReshapeAspectRatio_addr, ReshapeAspectRatio);
    new_hook!(KeepAspectRatio_addr, KeepAspectRatio);

    unsafe {
        WINDOW_LAST_WIDTH_FIELD = get_field_from_name(StandaloneWindowResize, c"windowLastWidth");
        WINDOW_LAST_HEIGHT_FIELD = get_field_from_name(StandaloneWindowResize, c"windowLastHeight");
        ASPECT_RATIO_FIELD = get_field_from_name(StandaloneWindowResize, c"_aspectRatio");
        IS_PREVENT_RESHAPE_FIELD = get_field_from_name(StandaloneWindowResize, c"_isPreventReShape");
        IS_VIRT_FIELD = get_field_from_name(StandaloneWindowResize, c"_isVirt");
        IS_WINDOW_SIZE_CHANGING_FIELD = get_field_from_name(StandaloneWindowResize, c"_isWindowSizeChanging");
        IS_WINDOW_DRAGGING_FIELD = get_field_from_name(StandaloneWindowResize, c"_isWindowDragging");

        SAVE_CHANGED_WIDTH_ADDR = get_method_addr(StandaloneWindowResize, c"SaveChangedWidth", 2);
        ENABLE_WINDOW_HIT_TEST_ADDR = get_method_addr(StandaloneWindowResize, c"EnableWindowHitTest", 0);
    }
}

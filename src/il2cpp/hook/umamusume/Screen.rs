use crate::il2cpp::{symbols::get_method_addr, types::*};

#[cfg(target_os = "android")]
use crate::core::Hachimi;

#[cfg(target_os = "windows")]
use std::ptr::null_mut;

#[cfg(target_os = "windows")]
use crate::{
    core::Hachimi,
    il2cpp::{
        api::il2cpp_field_static_set_value,
        hook::UnityEngine_CoreModule::Screen as UnityScreen,
        symbols::{get_field_from_name, IEnumerator, MoveNextFn},
    },
};

#[cfg(target_os = "android")]
extern "C" fn ChangeScreenOrientationLandscapeAsync_MoveNext(enumerator: *mut Il2CppObject) -> bool {
    use crate::il2cpp::symbols::MoveNextFn;
    let moved = get_orig_fn!(ChangeScreenOrientationLandscapeAsync_MoveNext, MoveNextFn)(enumerator);
    if !moved {
        super::UIManager::apply_ui_scale();
    }
    moved
}

#[cfg(target_os = "android")]
extern "C" fn ChangeScreenOrientationPortraitAsync_MoveNext(enumerator: *mut Il2CppObject) -> bool {
    use crate::il2cpp::symbols::MoveNextFn;
    let moved = get_orig_fn!(ChangeScreenOrientationPortraitAsync_MoveNext, MoveNextFn)(enumerator);
    if !moved {
        super::UIManager::apply_ui_scale();
    }
    moved
}

#[cfg(target_os = "android")]
type ChangeScreenOrientationLandscapeAsyncFn = extern "C" fn() -> crate::il2cpp::symbols::IEnumerator;
#[cfg(target_os = "android")]
extern "C" fn ChangeScreenOrientationLandscapeAsync() -> crate::il2cpp::symbols::IEnumerator {
    let enumerator = get_orig_fn!(ChangeScreenOrientationLandscapeAsync, ChangeScreenOrientationLandscapeAsyncFn)();
    if Hachimi::instance().config.load().ui_scale == 1.0 { return enumerator; }

    if let Err(e) = enumerator.hook_move_next(ChangeScreenOrientationLandscapeAsync_MoveNext) {
        error!("Failed to hook enumerator: {}", e);
    }

    enumerator
}

#[cfg(target_os = "android")]
type ChangeScreenOrientationPortraitAsyncFn = extern "C" fn() -> crate::il2cpp::symbols::IEnumerator;
#[cfg(target_os = "android")]
extern "C" fn ChangeScreenOrientationPortraitAsync() -> crate::il2cpp::symbols::IEnumerator {
    let enumerator = get_orig_fn!(ChangeScreenOrientationPortraitAsync, ChangeScreenOrientationPortraitAsyncFn)();
    if Hachimi::instance().config.load().ui_scale == 1.0 { return enumerator; }

    if let Err(e) = enumerator.hook_move_next(ChangeScreenOrientationPortraitAsync_MoveNext) {
        error!("Failed to hook enumerator: {}", e);
    }

    enumerator
}

#[cfg(target_os = "windows")]
type GetWidthFn = extern "C" fn() -> i32;
#[cfg(target_os = "windows")]
extern "C" fn get_Width() -> i32 {
    if Hachimi::instance().config.load().windows.freeform_window {
        return UnityScreen::get_width();
    }

    if let Some((width, _)) = crate::windows::utils::get_scaling_res() {
        return width;
    }

    get_orig_fn!(get_Width, GetWidthFn)()
}

#[cfg(target_os = "windows")]
pub fn get_Width_orig() -> i32 {
    get_orig_fn!(get_Width, GetWidthFn)()
}

#[cfg(target_os = "windows")]
type GetHeightFn = extern "C" fn() -> i32;
#[cfg(target_os = "windows")]
extern "C" fn get_Height() -> i32 {
    if Hachimi::instance().config.load().windows.freeform_window {
        return UnityScreen::get_height();
    }

    if let Some((_, height)) = crate::windows::utils::get_scaling_res() {
        return height;
    }

    get_orig_fn!(get_Height, GetHeightFn)()
}

#[cfg(target_os = "windows")]
pub fn get_Height_orig() -> i32 {
    get_orig_fn!(get_Height, GetHeightFn)()
}

#[cfg(target_os = "windows")]
static mut ORIGINAL_SCREEN_WIDTH_FIELD: *mut FieldInfo = null_mut();
#[cfg(target_os = "windows")]
static mut ORIGINAL_SCREEN_HEIGHT_FIELD: *mut FieldInfo = null_mut();

#[cfg(target_os = "windows")]
fn set_static_field<T>(field: *mut FieldInfo, value: T) {
    if field.is_null() {
        return;
    }

    il2cpp_field_static_set_value(field, std::ptr::from_ref(&value) as _);
}

#[cfg(target_os = "windows")]
pub fn update_original_screen_size(width: i32, height: i32) {
    let is_portrait = width < height;
    unsafe {
        set_static_field(ORIGINAL_SCREEN_WIDTH_FIELD, if is_portrait { height } else { width });
        set_static_field(ORIGINAL_SCREEN_HEIGHT_FIELD, if is_portrait { width } else { height });
    }
}

#[cfg(target_os = "windows")]
type SetResolutionFn = extern "C" fn(width: i32, height: i32, fullscreen: bool, force_update: bool);
#[cfg(target_os = "windows")]
extern "C" fn SetResolution(width: i32, height: i32, fullscreen: bool, force_update: bool) {
    if !Hachimi::instance().config.load().windows.freeform_window {
        get_orig_fn!(SetResolution, SetResolutionFn)(width, height, fullscreen, force_update);
    }
}

#[cfg(target_os = "windows")]
type SetResolution2Fn = extern "C" fn(width: i32, height: i32, fullscreen: bool, force_update: bool, skip_keep_aspect: bool);
#[cfg(target_os = "windows")]
extern "C" fn SetResolution2(width: i32, height: i32, fullscreen: bool, force_update: bool, skip_keep_aspect: bool) {
    if !Hachimi::instance().config.load().windows.freeform_window {
        get_orig_fn!(SetResolution2, SetResolution2Fn)(width, height, fullscreen, force_update, skip_keep_aspect);
    }
}

#[cfg(target_os = "windows")]
type IsCurrentOrientationFn = extern "C" fn(target: ScreenOrientation) -> bool;
#[cfg(target_os = "windows")]
extern "C" fn IsCurrentOrientation(target: ScreenOrientation) -> bool {
    if Hachimi::instance().config.load().windows.freeform_window {
        return true;
    }

    get_orig_fn!(IsCurrentOrientation, IsCurrentOrientationFn)(target)
}

#[cfg(target_os = "windows")]
type WaitDeviceOrientationFn = extern "C" fn(target: ScreenOrientation) -> IEnumerator;
#[cfg(target_os = "windows")]
extern "C" fn WaitDeviceOrientation(target: ScreenOrientation) -> IEnumerator {
    let enumerator = get_orig_fn!(WaitDeviceOrientation, WaitDeviceOrientationFn)(target);
    if Hachimi::instance().config.load().windows.freeform_window {
        if let Err(e) = enumerator.hook_move_next(WaitDeviceOrientation_MoveNext) {
            error!("Failed to stop WaitDeviceOrientation: {}", e);
        }
    }
    enumerator
}

#[cfg(target_os = "windows")]
extern "C" fn WaitDeviceOrientation_MoveNext(_enumerator: *mut Il2CppObject) -> bool {
    if Hachimi::instance().config.load().windows.freeform_window {
        return false;
    }

    get_orig_fn!(WaitDeviceOrientation_MoveNext, MoveNextFn)(_enumerator)
}

#[cfg(target_os = "windows")]
type ChangeScreenOrientationFn = extern "C" fn(target: ScreenOrientation, force: bool) -> IEnumerator;
#[cfg(target_os = "windows")]
extern "C" fn ChangeScreenOrientation(target: ScreenOrientation, force: bool) -> IEnumerator {
    let enumerator = get_orig_fn!(ChangeScreenOrientation, ChangeScreenOrientationFn)(target, force);
    if Hachimi::instance().config.load().windows.freeform_window {
        if let Err(e) = enumerator.hook_move_next(ChangeScreenOrientation_MoveNext) {
            error!("Failed to stop ChangeScreenOrientation: {}", e);
        }
    }
    enumerator
}

#[cfg(target_os = "windows")]
extern "C" fn ChangeScreenOrientation_MoveNext(_enumerator: *mut Il2CppObject) -> bool {
    if Hachimi::instance().config.load().windows.freeform_window {
        return false;
    }

    get_orig_fn!(ChangeScreenOrientation_MoveNext, MoveNextFn)(_enumerator)
}

#[cfg(target_os = "windows")]
type ChangeScreenOrientationAsyncFn = extern "C" fn() -> IEnumerator;
#[cfg(target_os = "windows")]
extern "C" fn ChangeScreenOrientationLandscapeAsyncWindows() -> IEnumerator {
    let enumerator = get_orig_fn!(
        ChangeScreenOrientationLandscapeAsyncWindows,
        ChangeScreenOrientationAsyncFn
    )();
    if Hachimi::instance().config.load().windows.freeform_window {
        if let Err(e) = enumerator.hook_move_next(ChangeScreenOrientationLandscapeAsyncWindows_MoveNext) {
            error!("Failed to stop landscape orientation change: {}", e);
        }
    }
    enumerator
}

#[cfg(target_os = "windows")]
extern "C" fn ChangeScreenOrientationLandscapeAsyncWindows_MoveNext(_enumerator: *mut Il2CppObject) -> bool {
    if Hachimi::instance().config.load().windows.freeform_window {
        return false;
    }

    get_orig_fn!(ChangeScreenOrientationLandscapeAsyncWindows_MoveNext, MoveNextFn)(_enumerator)
}

#[cfg(target_os = "windows")]
extern "C" fn ChangeScreenOrientationPortraitAsyncWindows() -> IEnumerator {
    let enumerator = get_orig_fn!(
        ChangeScreenOrientationPortraitAsyncWindows,
        ChangeScreenOrientationAsyncFn
    )();
    if Hachimi::instance().config.load().windows.freeform_window {
        if let Err(e) = enumerator.hook_move_next(ChangeScreenOrientationPortraitAsyncWindows_MoveNext) {
            error!("Failed to stop portrait orientation change: {}", e);
        }
    }
    enumerator
}

#[cfg(target_os = "windows")]
extern "C" fn ChangeScreenOrientationPortraitAsyncWindows_MoveNext(_enumerator: *mut Il2CppObject) -> bool {
    if Hachimi::instance().config.load().windows.freeform_window {
        return false;
    }

    get_orig_fn!(ChangeScreenOrientationPortraitAsyncWindows_MoveNext, MoveNextFn)(_enumerator)
}

static mut GET_ISSPLITWINDOW_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_IsSplitWindow, GET_ISSPLITWINDOW_ADDR, bool,);

static mut GET_ISVERTICAL_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_IsVertical, GET_ISVERTICAL_ADDR, bool,);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, Screen);

    #[cfg(target_os = "android")]
    {
        let ChangeScreenOrientationLandscapeAsync_addr = get_method_addr(Screen, c"ChangeScreenOrientationLandscapeAsync", 0);
        let ChangeScreenOrientationPortraitAsync_addr = get_method_addr(Screen, c"ChangeScreenOrientationPortraitAsync", 0);

        new_hook!(ChangeScreenOrientationLandscapeAsync_addr, ChangeScreenOrientationLandscapeAsync);
        new_hook!(ChangeScreenOrientationPortraitAsync_addr, ChangeScreenOrientationPortraitAsync);
    }

    #[cfg(target_os = "windows")]
    {
        let get_Width_addr = get_method_addr(Screen, c"get_Width", 0);
        let get_Height_addr = get_method_addr(Screen, c"get_Height", 0);
        let SetResolution_addr = get_method_addr(Screen, c"SetResolution", 4);
        let SetResolution2_addr = get_method_addr(Screen, c"SetResolution", 5);
        let IsCurrentOrientation_addr = get_method_addr(Screen, c"IsCurrentOrientation", 1);
        let WaitDeviceOrientation_addr = get_method_addr(Screen, c"WaitDeviceOrientation", 1);
        let ChangeScreenOrientation_addr = get_method_addr(Screen, c"ChangeScreenOrientation", 2);
        let ChangeScreenOrientationLandscapeAsyncWindows_addr =
            get_method_addr(Screen, c"ChangeScreenOrientationLandscapeAsync", 0);
        let ChangeScreenOrientationPortraitAsyncWindows_addr =
            get_method_addr(Screen, c"ChangeScreenOrientationPortraitAsync", 0);

        new_hook!(get_Width_addr, get_Width);
        new_hook!(get_Height_addr, get_Height);
        new_hook!(SetResolution_addr, SetResolution);
        new_hook!(SetResolution2_addr, SetResolution2);
        new_hook!(IsCurrentOrientation_addr, IsCurrentOrientation);
        new_hook!(WaitDeviceOrientation_addr, WaitDeviceOrientation);
        new_hook!(ChangeScreenOrientation_addr, ChangeScreenOrientation);
        new_hook!(
            ChangeScreenOrientationLandscapeAsyncWindows_addr,
            ChangeScreenOrientationLandscapeAsyncWindows
        );
        new_hook!(
            ChangeScreenOrientationPortraitAsyncWindows_addr,
            ChangeScreenOrientationPortraitAsyncWindows
        );

        unsafe {
            ORIGINAL_SCREEN_WIDTH_FIELD = get_field_from_name(Screen, c"_originalScreenWidth");
            ORIGINAL_SCREEN_HEIGHT_FIELD = get_field_from_name(Screen, c"_originalScreenHeight");
        }
    }

    unsafe {
        GET_ISSPLITWINDOW_ADDR = get_method_addr(Screen, c"get_IsSplitWindow", 0);
        GET_ISVERTICAL_ADDR = get_method_addr(Screen, c"get_IsVertical", 0);
    }
}

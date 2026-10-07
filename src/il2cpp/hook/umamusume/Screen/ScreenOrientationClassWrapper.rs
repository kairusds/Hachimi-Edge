use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut SET_ORIENTATION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_Orientation, SET_ORIENTATION_ADDR, (), this: *mut Il2CppObject, value: ScreenOrientation);

static mut GET_ORIENTATION_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_Orientation, GET_ORIENTATION_ADDR, ScreenOrientation, this: *mut Il2CppObject);

#[cfg(target_os = "android")]
type set_OrientationHookFn = extern "C" fn(this: *mut Il2CppObject, value: i32);
#[cfg(target_os = "android")]
extern "C" fn set_OrientationHook(this: *mut Il2CppObject, value: i32) {
    use crate::{
        core::Hachimi,
        il2cpp::hook::umamusume::Screen::should_force_orientation
    };

    if should_force_orientation() {
        let force_orientation = Hachimi::instance().config.load().android.force_orientation_mode;
        get_orig_fn!(set_OrientationHook, set_OrientationHookFn)(this, force_orientation);
    } else {
        get_orig_fn!(set_OrientationHook, set_OrientationHookFn)(this, value);
    };
}

pub fn init(Screen: *mut Il2CppClass) {
    find_nested_class_or_return!(Screen, ScreenOrientationClassWrapper);

    unsafe {
        SET_ORIENTATION_ADDR = get_method_addr(ScreenOrientationClassWrapper, c"set_Orientation", 1);
        GET_ORIENTATION_ADDR = get_method_addr(ScreenOrientationClassWrapper, c"get_Orientation", 0);

        #[cfg(target_os = "android")]
        {
            new_hook!(SET_ORIENTATION_ADDR, set_OrientationHook);
        }
    }
}

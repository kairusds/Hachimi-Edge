use crate::{
    core::Hachimi,
    il2cpp::{
        hook::UnityEngine_InputLegacyModule::Input,
        symbols::get_method_addr,
        types::*,
    },
};

use super::WindowsGamepadControl;

type MousePositionFn = extern "C" fn() -> Vector3_t;
extern "C" fn mousePosition() -> Vector3_t {
    if !Hachimi::instance().config.load().windows.freeform_window {
        return get_orig_fn!(mousePosition, MousePositionFn)();
    }

    WindowsGamepadControl::update_input_controls();
    Input::mouse_position()
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, GallopInput);

    let mousePosition_addr = get_method_addr(GallopInput, c"mousePosition", 0);
    new_hook!(mousePosition_addr, mousePosition);
}

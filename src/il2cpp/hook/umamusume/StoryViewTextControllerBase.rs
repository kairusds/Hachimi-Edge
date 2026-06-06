use super::{StoryViewTextControllerLandscape, StoryViewTextControllerSingleMode, TextFrame};
use crate::il2cpp::{ext::Il2CppObjectExt, hook::UnityEngine_UI::Text, symbols::get_method_addr, types::*, utils};
use std::ptr::null_mut;

static mut CLASS_SINGLE: *mut Il2CppClass = null_mut();
static mut CLASS_LANDSCAPE: *mut Il2CppClass = null_mut();

type Size = (f32, f32);
const SINGLEMODE_SIZE: Size = (450.0, 65.0);
const LANDSCAPE_SIZE: Size = (730.0, 45.0);

type SetNameLabelFn = extern "C" fn(this: *mut Il2CppObject, name: *mut Il2CppString);
extern "C" fn SetNameLabel(this: *mut Il2CppObject, name: *mut Il2CppString) {
    let cls = unsafe { (*this).klass() };
    let text_frame: *mut Il2CppObject;
    let size: &Size;

    if cls == unsafe { CLASS_SINGLE } {
        text_frame = StoryViewTextControllerSingleMode::get__textFrame(this);
        size = &SINGLEMODE_SIZE;
    } else if cls == unsafe { CLASS_LANDSCAPE } {
        text_frame = StoryViewTextControllerLandscape::get__textFrame(this);
        size = &LANDSCAPE_SIZE;
    } else {
        return get_orig_fn!(SetNameLabel, SetNameLabelFn)(this, name);
    };

    let name_label = TextFrame::get_NameLabel(text_frame);
    utils::adjust_transform_size(name_label, size.0, size.1);
    Text::set_best_fit_downscale(name_label);
    Text::set_horizontalOverflow(name_label, TextOverflow_Disallow);
    get_orig_fn!(SetNameLabel, SetNameLabelFn)(this, name);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StoryViewTextControllerSingleMode);
    get_class_or_return!(umamusume, Gallop, StoryViewTextControllerLandscape);

    // Actually is abstract in Base class, but overrides are the same function for at least SingleMode & Landscape.
    let SetNameLabel_addr = get_method_addr(StoryViewTextControllerSingleMode, c"SetNameLabel", 1);
    new_hook!(SetNameLabel_addr, SetNameLabel);

    unsafe {
        CLASS_SINGLE = StoryViewTextControllerSingleMode;
        CLASS_LANDSCAPE = StoryViewTextControllerLandscape;
    }
}

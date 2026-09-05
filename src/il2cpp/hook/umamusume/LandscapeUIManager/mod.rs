use crate::il2cpp::{symbols::get_method_addr, types::*};

pub mod ScreenInfo;

def_method_wrapper_fn!(get_GameScreenInfo, GET_GAMESCREENINFO_ADDR, *mut Il2CppObject,);
def_method_wrapper_fn!(get_WindowScaleRate, GET_WINDOWSCALE_RATE_ADDR, f32,);

// (split, offset_x, offset_y, size_w, size_h) in game view px, or None
pub fn game_screen_info() -> Option<(f32, f32, f32, f32)> {
    let info = get_GameScreenInfo();
    if info.is_null() {
        return None;
    }

    let size = ScreenInfo::get_Size(info);
    let offset = ScreenInfo::get_OffsetPos(info);
    Some((offset.x, offset.y, size.x, size.y))
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, LandscapeUIManager);

    ScreenInfo::init(LandscapeUIManager);

    unsafe {
        GET_GAMESCREENINFO_ADDR = get_method_addr(LandscapeUIManager, c"get_GameScreenInfo", 0);
        GET_WINDOWSCALE_RATE_ADDR = get_method_addr(LandscapeUIManager, c"get_WindowScaleRate", 0);
    }
}

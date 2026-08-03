use crate::{core::Hachimi, il2cpp::{symbols::get_method_addr, types::*}};
use super::GameDefine::BgSeason;

// public static BgSeason GetSeasonForHome(DateTime dateTime) { }
type GetSeasonForHomeFn = extern "C" fn(this: *mut Il2CppObject, dateTime: *mut Il2CppObject) -> BgSeason;
extern "C" fn GetSeasonForHome(this: *mut Il2CppObject, dateTime: *mut Il2CppObject) -> BgSeason {
    let bg_season = Hachimi::instance().config.load().homescreen_bgseason;
    if bg_season != BgSeason::None {
        return bg_season;
    }
    get_orig_fn!(GetSeasonForHome, GetSeasonForHomeFn)(this, dateTime)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TimeUtil);
    
    let GetSeasonForHome_addr = get_method_addr(TimeUtil, c"GetSeasonForHome", 1);
    new_hook!(GetSeasonForHome_addr, GetSeasonForHome);
}

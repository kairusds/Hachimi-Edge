use crate::{core::Hachimi, il2cpp::{symbols::get_method_addr, types::*}};
use super::{
    GameDefine::BgSeason,
    MasterDataManager,
    MasterItemExchangeTop
};

// public static BgSeason GetSeasonForHome(DateTime dateTime) { }
type GetSeasonForHomeFn = extern "C" fn(dateTime: *mut Il2CppObject) -> BgSeason;
extern "C" fn GetSeasonForHome(dateTime: *mut Il2CppObject) -> BgSeason {
    let orig = get_orig_fn!(GetSeasonForHome, GetSeasonForHomeFn)(dateTime);
    let bg_season = Hachimi::instance().config.load().homescreen_bgseason;
    let master_mgr = MasterDataManager::instance();

    if master_mgr.is_null() || bg_season == BgSeason::None {
        return orig;
    }

    let master_itex_top = MasterDataManager::get_masterItemExchangeTop(master_mgr);
    if master_itex_top.is_null() { return orig; }

    // Overriding BgSeason during anniversary & half anniversary breaks the game so this has to be gated
    let in_term_anniv_shop = MasterItemExchangeTop::get_IsInTermAnyAnnivShop(master_itex_top);
    debug!("in_term_anniv_shop {}", in_term_anniv_shop);
    if !in_term_anniv_shop && bg_season > BgSeason::None && bg_season <= BgSeason::CherryBlossom {
        bg_season
    } else {
        orig
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TimeUtil);
    
    let GetSeasonForHome_addr = get_method_addr(TimeUtil, c"GetSeasonForHome", 1);
    new_hook!(GetSeasonForHome_addr, GetSeasonForHome);
}

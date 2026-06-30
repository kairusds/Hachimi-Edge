use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GET_TEMPSETLISTPLAYINGDATA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_TempSetListPlayingData, GET_TEMPSETLISTPLAYINGDATA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut SETPLAYMUSICFLAG_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetPlayMusicFlag, SETPLAYMUSICFLAG_ADDR, (), this: *mut Il2CppObject, flag: bool);

static mut PLAYREQUESTSONG_ADDR: usize = 0;
impl_addr_wrapper_fn!(PlayRequestSong, PLAYREQUESTSONG_ADDR, (), this: *mut Il2CppObject);

static mut ONCLICKSETLISTARROW_ADDR: usize = 0;
impl_addr_wrapper_fn!(OnClickSetListArrow, ONCLICKSETLISTARROW_ADDR, (), this: *mut Il2CppObject, is_next: bool);

static mut GET_JUKEBOXBGMSELECTOR_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_JukeboxBgmSelector, GET_JUKEBOXBGMSELECTOR_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, JukeboxHomeTopUI);

    unsafe {
        CLASS = JukeboxHomeTopUI;
        GET_TEMPSETLISTPLAYINGDATA_ADDR = get_method_addr(JukeboxHomeTopUI, c"get_TempSetListPlayingData", 0);
        SETPLAYMUSICFLAG_ADDR = get_method_addr(JukeboxHomeTopUI, c"SetPlayMusicFlag", 1);
        PLAYREQUESTSONG_ADDR = get_method_addr(JukeboxHomeTopUI, c"PlayRequestSong", 0);
        ONCLICKSETLISTARROW_ADDR = get_method_addr(JukeboxHomeTopUI, c"OnClickSetListArrow", 1);
        GET_JUKEBOXBGMSELECTOR_ADDR = get_method_addr(JukeboxHomeTopUI, c"get_JukeboxBgmSelector", 0);
    }
}

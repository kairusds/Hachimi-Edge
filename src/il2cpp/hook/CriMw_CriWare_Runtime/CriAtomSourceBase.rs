use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;

pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

// public CriAtomExPlayer get_player() { }
static mut GET_PLAYER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_player, GET_PLAYER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(CriMw_CriWare_Runtime: *const Il2CppImage) {
    get_class_or_return!(CriMw_CriWare_Runtime, CriWare, CriAtomSourceBase);

    unsafe {
        CLASS = CriAtomSourceBase;
        GET_PLAYER_ADDR = get_method_addr(CriAtomSourceBase, c"get_player", 0);
    }
}

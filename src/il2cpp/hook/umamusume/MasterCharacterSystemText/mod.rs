use crate::{
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
};

pub mod CharacterSystemText;

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GETBYCHARAID_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetByCharaId, GETBYCHARAID_ADDR, *mut Il2CppObject, chara_id: i32);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, MasterCharacterSystemText);

    unsafe {
        CLASS = MasterCharacterSystemText;
        GETBYCHARAID_ADDR = get_method_addr(MasterCharacterSystemText, c"GetByCharaId", 1);
    }

    CharacterSystemText::init(MasterCharacterSystemText);
}
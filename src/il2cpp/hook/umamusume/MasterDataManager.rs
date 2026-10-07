use crate::{
    il2cpp::{
        symbols::{get_method_addr, SingletonLike},
        types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

// public MasterItemExchangeTop get_masterItemExchangeTop() { }
static mut GET_MASTERITEMEXCHANGETOP_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_masterItemExchangeTop, GET_MASTERITEMEXCHANGETOP_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, MasterDataManager);

    unsafe {
        CLASS = MasterDataManager;

        GET_MASTERITEMEXCHANGETOP_ADDR = get_method_addr(MasterDataManager, c"get_masterItemExchangeTop", 0);
    }
}

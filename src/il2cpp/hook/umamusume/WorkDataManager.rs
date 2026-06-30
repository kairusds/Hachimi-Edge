use crate::il2cpp::{
    symbols::{get_method_addr, SingletonLike},
    types::*
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

static mut GET_JUKEBOXDATA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_JukeboxData, GET_JUKEBOXDATA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, WorkDataManager);

    unsafe {
        CLASS = WorkDataManager;
        GET_JUKEBOXDATA_ADDR = get_method_addr(WorkDataManager, c"get_JukeboxData", 0);
    }
}

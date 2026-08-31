use crate::{
    core::Hachimi,
    il2cpp::{
        symbols::{get_method_addr, SingletonLike},
        types::*,
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

static mut REFRESH_ALL_ADDR: usize = 0;
impl_addr_wrapper_fn!(RefreshAll, REFRESH_ALL_ADDR, (), this: *mut Il2CppObject);

type UpdateFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Update(this: *mut Il2CppObject) {
    if Hachimi::instance().config.load().disable_tap_effect {
        return;
    }
    get_orig_fn!(Update, UpdateFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TapEffectController);

    let Update_addr = get_method_addr(TapEffectController, c"Update", 0);
    new_hook!(Update_addr, Update);

    unsafe {
        CLASS = TapEffectController;
        REFRESH_ALL_ADDR = get_method_addr(TapEffectController, c"RefreshAll", 0);
    }
}

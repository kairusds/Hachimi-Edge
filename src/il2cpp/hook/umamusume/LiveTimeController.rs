use crate::{
    il2cpp::{
        symbols::{get_method_addr, get_field_from_name, get_class},
        types::*
    }
};

static mut PAUSELIVE_TC_ADDR: usize = 0;
impl_addr_wrapper_fn!(PauseLive_TC, PAUSELIVE_TC_ADDR, (), this: *mut Il2CppObject);

static mut RESUMELIVE_TC_ADDR: usize = 0;
impl_addr_wrapper_fn!(ResumeLive_TC, RESUMELIVE_TC_ADDR, (), this: *mut Il2CppObject);

static mut SET_CURRENTTIME_TC_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_CurrentTime_TC, SET_CURRENTTIME_TC_ADDR, (), this: *mut Il2CppObject, time: f32);

def_field_value_accessors!(set set__elapsedTime_TC, _ELAPSEDTIME_TC_FIELD, f32);

pub fn init(umamusume: *const Il2CppImage) {
    let ltc_class = get_class(umamusume, c"Gallop.Live", c"LiveTimeController")
        .or_else(|_| get_class(umamusume, c"Gallop", c"LiveTimeController"));

    if let Ok(ltc) = ltc_class {
        unsafe {
            PAUSELIVE_TC_ADDR = get_method_addr(ltc, c"PauseLive", 0);
            RESUMELIVE_TC_ADDR = get_method_addr(ltc, c"ResumeLive", 0);
            SET_CURRENTTIME_TC_ADDR = get_method_addr(ltc, c"set_CurrentTime", 1);
            _ELAPSEDTIME_TC_FIELD = get_field_from_name(ltc, c"_elapsedTime");
        }
    } else {
        error!("Failed to resolve LiveTimeController class, live slider time control will be unavailable");
    }
}

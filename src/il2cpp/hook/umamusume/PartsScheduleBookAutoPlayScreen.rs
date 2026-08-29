use crate::{
    core::taskbar::{self, TBPF_NORMAL, TBPF_NOPROGRESS},
    il2cpp::{
        symbols::{get_field_from_name, get_field_value, get_method_addr}, types::{FieldInfo, Il2CppImage, Il2CppObject},
    },
};

static mut GAUGE_COUNT_FIELD: *mut FieldInfo = 0 as _;
fn get__progressGaugeCount(this: *mut Il2CppObject) -> f32 {
    get_field_value(this, unsafe { GAUGE_COUNT_FIELD })
}
static mut GAUGE_MAX_FIELD: *mut FieldInfo = 0 as _;
fn get__progressGaugeMax(this: *mut Il2CppObject) -> f32 {
    get_field_value(this, unsafe { GAUGE_MAX_FIELD })
}

type ShowScreenFn = extern "C" fn(this: *mut Il2CppObject, progress_max: i32, need_stop: bool);
extern "C" fn ShowScreen(this: *mut Il2CppObject, progress_max: i32, need_stop: bool) {
    taskbar::update_schedule_state(TBPF_NORMAL, true);
    taskbar::update_schedule_value(0, progress_max as u64);
    get_orig_fn!(ShowScreen, ShowScreenFn)(this, progress_max, need_stop);
}

type HideScreenFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn HideScreen(this: *mut Il2CppObject) {
    taskbar::update_schedule_state(TBPF_NOPROGRESS, false);
    get_orig_fn!(HideScreen, HideScreenFn)(this);
}

type IncrementProgressGaugeFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn IncrementProgressGauge(this: *mut Il2CppObject) {
    get_orig_fn!(IncrementProgressGauge, IncrementProgressGaugeFn)(this);
    taskbar::update_schedule_value(
        get__progressGaugeCount(this) as u64,
        get__progressGaugeMax(this) as u64
    );
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsScheduleBookAutoPlayScreen);

    let show_addr = get_method_addr(PartsScheduleBookAutoPlayScreen, c"ShowScreen", 2);
    let hide_addr = get_method_addr(PartsScheduleBookAutoPlayScreen, c"HideScreen", 0);
    let increment_addr = get_method_addr(PartsScheduleBookAutoPlayScreen, c"IncrementProgressGauge", 0);

    new_hook!(show_addr, ShowScreen);
    new_hook!(hide_addr, HideScreen);
    new_hook!(increment_addr, IncrementProgressGauge);

    unsafe {
        GAUGE_COUNT_FIELD = get_field_from_name(PartsScheduleBookAutoPlayScreen, c"_progressGaugeCount");
        GAUGE_MAX_FIELD = get_field_from_name(PartsScheduleBookAutoPlayScreen, c"_progressGaugeMax");
    }
}
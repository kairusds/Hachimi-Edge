use std::sync::atomic::{self, AtomicBool};

use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        symbols::{get_method_addr, get_field_from_name},
        types::*
    }
};

def_field_value_accessors!(get__choiceAutoSelectWaitTime, set__choiceAutoSelectWaitTime, _CHOICEAUTOSELECTWAITTIME_FIELD, f32);

static IS_CHECKING_CHOICE_AUTO_TAP: AtomicBool = AtomicBool::new(false);
pub fn is_checking_choice_auto_tap() -> bool {
    IS_CHECKING_CHOICE_AUTO_TAP.swap(false, atomic::Ordering::Relaxed)
}

type CheckChoiceAutoTapFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn CheckChoiceAutoTap(this: *mut Il2CppObject) {
    IS_CHECKING_CHOICE_AUTO_TAP.store(true, atomic::Ordering::Relaxed);

    // Global has a different way of handling choice auto select delay in stories
    let is_global = Hachimi::instance().game.region == Region::Global;
    let delay = Hachimi::instance().config.load().story_choice_auto_select_delay;
    let needs_scaling = is_global && delay != 0.75 && delay > 0.0 && delay.is_finite();
    let before = if needs_scaling {
        get__choiceAutoSelectWaitTime(this)
    } else {
        0.0
    };

    get_orig_fn!(CheckChoiceAutoTap, CheckChoiceAutoTapFn)(this);

    if needs_scaling {
        let after = get__choiceAutoSelectWaitTime(this);
        let increment = after - before;
        if increment > 0.0 {
            // _choiceAutoSelectWaitTime accumulates elapsed time upward from 0
            // Auto select triggers when it reaches SINGLE_CHOICE_AUTO_SELECT_DURATION (0.75)
            // Scale the increment so it takes "delay" seconds instead of 0.75
            let mult = 0.75 / delay;
            set__choiceAutoSelectWaitTime(this, before + increment * mult);
        }
    }

    IS_CHECKING_CHOICE_AUTO_TAP.store(false, atomic::Ordering::Relaxed);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StoryChoiceController);

    unsafe {
        _CHOICEAUTOSELECTWAITTIME_FIELD = get_field_from_name(StoryChoiceController, c"_choiceAutoSelectWaitTime");
    }

    let CheckChoiceAutoTap_addr = get_method_addr(StoryChoiceController, c"CheckChoiceAutoTap", 0);
    new_hook!(CheckChoiceAutoTap_addr, CheckChoiceAutoTap);
}
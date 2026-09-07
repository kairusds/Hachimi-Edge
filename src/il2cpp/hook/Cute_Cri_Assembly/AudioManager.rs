use crate::il2cpp::{
    symbols::{get_field_from_name, get_method_addr, Dictionary, Il2CppDictionary},
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;

pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_field_object_accessors!(get get_audioCtrlDictRaw, AUDIOCTRLDICT_FIELD, Il2CppDictionary);

pub fn get_audioCtrlDict(this: *mut Il2CppObject) -> Dictionary<i32, *mut Il2CppObject> {
    Dictionary::from(get_audioCtrlDictRaw(this))
}

// public Void StopAll(SoundGroup group, Single fadeOutTime, FadeCurve fadeCurve, Action stoppedCallback) { }
def_method_wrapper_fn!(StopAll, STOPALL_ADDR, (), this: *mut Il2CppObject, group: i32, fade_out_time: f32, fade_curve: i32, stopped_callback: *mut Il2CppObject);

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", AudioManager);

    unsafe {
        CLASS = AudioManager;
        AUDIOCTRLDICT_FIELD = get_field_from_name(AudioManager, c"audioCtrlDict");
        STOPALL_ADDR = get_method_addr(AudioManager, c"StopAll", 4);
    }
}

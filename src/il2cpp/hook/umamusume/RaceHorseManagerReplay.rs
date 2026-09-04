use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        api::il2cpp_class_is_assignable_from,
        ext::Il2CppObjectExt,
        symbols::get_field_from_name,
        types::*
    }
};

def_field_object_accessors!(get__reader, set__reader, READER_FIELD, Il2CppObject);

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn is_replay_manager(obj: *mut Il2CppObject) -> bool {
    if class().is_null() || obj.is_null() {
        return false;
    }

    let obj_class = unsafe { (*obj).klass() };
    !obj_class.is_null() && il2cpp_class_is_assignable_from(class(), obj_class)
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, RaceHorseManagerReplay);

    unsafe {
        CLASS = RaceHorseManagerReplay;

        READER_FIELD = get_field_from_name(RaceHorseManagerReplay, c"_reader");
    }
}

use crate::il2cpp::{symbols::{get_field_from_name}, types::*};

def_field_value_accessors!(get_MusicId, set_MusicId, MUSICID_FIELD, i32);

pub fn init(TempData: *mut Il2CppClass) {
    find_nested_class_or_return!(TempData, JukeboxSetlistMusicData);

    unsafe {
        MUSICID_FIELD = get_field_from_name(JukeboxSetlistMusicData, c"MusicId");
    }
}
use crate::il2cpp::{symbols::{get_field_from_name, get_method_addr}, types::*};

static mut GETMUSICLISTCOUNT_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetMusicListCount, GETMUSICLISTCOUNT_ADDR, i32, this: *mut Il2CppObject);

static mut GETMASTERSETLISTMUSICDATA_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetMasterSetListMusicData, GETMASTERSETLISTMUSICDATA_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

def_field_value_accessors!(get_IsPlaying, set_IsPlaying, ISPLAYING_FIELD, bool);
def_field_value_accessors!(get_SetListIndex, set_SetListIndex, SETLISTINDEX_FIELD, i32);
def_field_value_accessors!(get_SetListId, set_SetListId, SETLISTID_FIELD, i32);

pub fn init(TempData: *mut Il2CppClass) {
    find_nested_class_or_return!(TempData, JukeboxSetListPlayingData);

    unsafe {
        GETMUSICLISTCOUNT_ADDR = get_method_addr(JukeboxSetListPlayingData, c"GetMusicListCount", 0);
        GETMASTERSETLISTMUSICDATA_ADDR = get_method_addr(JukeboxSetListPlayingData, c"GetMasterSetListMusicData", 0);
        ISPLAYING_FIELD = get_field_from_name(JukeboxSetListPlayingData, c"IsPlaying");
        SETLISTINDEX_FIELD = get_field_from_name(JukeboxSetListPlayingData, c"SetListIndex");
        SETLISTID_FIELD = get_field_from_name(JukeboxSetListPlayingData, c"SetListId");
    }
}
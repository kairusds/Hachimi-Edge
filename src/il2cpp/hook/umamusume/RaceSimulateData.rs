use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        symbols::{get_class, get_field_from_name, get_method_addr},
        types::*
    }
};

def_method_wrapper_fn!(get_FrameDataList, GET_FRAME_DATA_LIST_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_field_object_accessors!(get__simEvDataList, set__simEvDataList, SIM_EV_DATA_LIST_FIELD, Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    if !matches!(Hachimi::instance().game.region, Region::Japan | Region::Global) {
        return;
    }

    let namespace = if Hachimi::instance().game.region == Region::Global { c"Gallop" } else { c"StandaloneSimulator" };
    let RaceSimulateData = match get_class(umamusume, namespace, c"RaceSimulateData") {
        Ok(v) => v,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };

    unsafe {
        GET_FRAME_DATA_LIST_ADDR = get_method_addr(RaceSimulateData, c"get_FrameDataList", 0);
        SIM_EV_DATA_LIST_FIELD = get_field_from_name(RaceSimulateData, c"_simEvDataList");
    }
}

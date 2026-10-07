use std::sync::atomic::Ordering;
use crate::{
    core::{Hachimi, game::Region, gui, utils::{clear_il2cpp_list, race_seek_seh, race_seek_stage}},
    il2cpp::{
        symbols::{get_field_from_name, get_method_addr, Array, IList},
        types::*
    }
};
use super::{
    HorseRaceInfo,
    RaceEventPlayer,
    RaceHorseManagerBase,
    RaceHorseManagerReplay,
    RaceManager,
    RaceSimulateEventData,
    RaceSimulateReader,
    Jikkyo,
    JikkyoControllerBase, 
    RaceMainViewController,
    RaceSimulateData,
    RaceUI,
    RaceUIMiniMap,
    RaceViewReplay,
    SimulateEventType,
    SkillManager,
};

def_method_wrapper_fn!(ForceSetRaceTime, FORCE_SET_RACE_TIME_ADDR, (), this: *mut Il2CppObject, time: f32, isCheckEvent: bool);
def_method_wrapper_fn!(IsPaused, IS_PAUSED_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(PauseRace, PAUSE_RACE_ADDR, (), this: *mut Il2CppObject);
def_method_wrapper_fn!(ResumeRace, RESUME_RACE_ADDR, (), this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsPlayingCutIn, IS_PLAYING_CUT_IN_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_Jikkyo, GET_JIKKYO_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(UpdateCameraEventAll, UPDATE_CAMERA_EVENT_ALL_ADDR, (), this: *mut Il2CppObject, force_update: bool);
def_method_wrapper_fn!(LateUpdateCameraEventAll, LATE_UPDATE_CAMERA_EVENT_ALL_ADDR, (), this: *mut Il2CppObject, force_update: bool);
def_method_wrapper_fn!(IsCutInPlayingOrReserved, IS_CUT_IN_PLAYING_OR_RESERVED_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(UpdateHorses, UPDATE_HORSES_ADDR, (), this: *mut Il2CppObject, elapsed_time: f32);
def_method_wrapper_fn!(UpdateHorseDatas, UPDATE_HORSE_DATAS_ADDR, (), this: *mut Il2CppObject, elapsed_time: f32);
def_method_wrapper_fn!(UpdateHorseModels, UPDATE_HORSE_MODELS_ADDR, (), this: *mut Il2CppObject);

def_field_object_accessors!(get__eventPlayer, set__eventPlayer, EVENT_PLAYER_FIELD, Il2CppObject);

pub fn seek_sync(target_time: f32) -> bool {
    let race_manager = RaceManager::instance();
    if race_manager.is_null() { return true; }

    let event_player = get__eventPlayer(race_manager);

    let prev_time = RaceSimulateReader::replay_cur_time(race_manager)
        .unwrap_or(f32::from_bits(gui::RACE_SLIDER_LAST_APPLIED.load(Ordering::Acquire)));
    let backward = target_time < prev_time - 0.5;

    let ok = race_seek_seh(|| {
        if RaceEventPlayer::is_event_player(event_player) {
            race_seek_stage(1); // event_cursor
            RaceEventPlayer::ChangeLastEventIndexByTime(event_player, target_time);
        }

        race_seek_stage(2); // race_time
        ForceSetRaceTime(race_manager, target_time, true);
        race_seek_stage(3); // horses
        UpdateHorses(race_manager, target_time);
        race_seek_stage(4); // horse_datas
        UpdateHorseDatas(race_manager, target_time);
        race_seek_stage(5); // models
        UpdateHorseModels(race_manager);
        race_seek_stage(6); // camera_events
        UpdateCameraEventAll(race_manager, true);
        race_seek_stage(7); // late_camera_events
        LateUpdateCameraEventAll(race_manager, true);
        race_seek_stage(8); // used_skills
        resync_used_skills(race_manager, target_time);

        race_seek_stage(9); // jikkyo_sync
        let jikkyo = get_Jikkyo(race_manager);
        if jikkyo.is_null() { return; }

        if backward {
            Jikkyo::SkipToStart(jikkyo);
        } else {
            JikkyoControllerBase::ClearDisplay(jikkyo);
            JikkyoControllerBase::ClearVoice(jikkyo);
            JikkyoControllerBase::ClearReserve(jikkyo);
        }

        race_seek_stage(10); // view_rearm
        let view = RaceManager::get_RaceView(race_manager);
        if view.is_null() { return; }
        RaceViewReplay::set_lastSpurtProcessed(view, false);

        if backward {
            race_seek_stage(11); // minimap_rearm
            let race_main_view = RaceManager::get_RaceMainView(race_manager);
            if race_main_view.is_null() { return; }

            let race_ui = RaceMainViewController::get__raceUI(race_main_view);
            if race_ui.is_null() { return; }

            let minimap = RaceUI::get__minimap(race_ui);
            if minimap.is_null() { return; }

            RaceUIMiniMap::set_hasMiniMapShown(minimap, false);
            RaceUIMiniMap::set_hasMiniMapHidden(minimap, false);
        }

        race_seek_stage(0); // idle
    });

    ok
}

fn resync_used_skills(race_manager: *mut Il2CppObject, target_time: f32) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    let horse_manager = RaceManager::get__horseManager(race_manager);
    if horse_manager.is_null() { return; }
    if !RaceHorseManagerReplay::is_replay_manager(horse_manager) { return; }

    let horse_infos = RaceHorseManagerBase::GetHorseRaceInfos(horse_manager);
    if horse_infos.is_null() { return; }
    let horse_arr: Array<*mut Il2CppObject> = Array::from(horse_infos);
    let horse_count = horse_arr.len();
    if horse_count == 0 { return; }

    let horse_infos_slice = unsafe { horse_arr.as_slice() };
    for horse_info in horse_infos_slice.iter() {
        let skill_manager = HorseRaceInfo::get__skillManager(*horse_info);
        if skill_manager.is_null() { continue; }
        let used_list = SkillManager::GetUsedSkillIdList(skill_manager);
        if used_list.is_null() { continue; }
        clear_il2cpp_list(used_list);
    }

    let reader = RaceHorseManagerReplay::get__reader(horse_manager);
    if reader.is_null() { return; }

    let sim_data = RaceSimulateReader::get__simData(reader);
    if sim_data.is_null() { return; }

    let event_list = RaceSimulateData::get__simEvDataList(sim_data);
    let Some(event_list) = IList::<*mut Il2CppObject>::new(event_list) else {
        return;
    };

    for event in event_list.iter() {
        if event.is_null() { continue; }
        if RaceSimulateEventData::get_type(event) != SimulateEventType::Skill { continue; }
        if RaceSimulateEventData::get_frameTime(event) > target_time { continue; }

        let mut horse_idx: i32 = -1;
        let mut skill_id: i32 = 0;
        let mut detail_index: i32 = 0;
        let mut time_int: i32 = 0;
        let mut target_flags: i32 = 0;
        let mut caller_skill_id: i32 = 0;
        let mut activate_type: i32 = 0;
        let mut ability_value_status: i32 = 0;
        let mut ability_time_status: i32 = 0;
        RaceEventPlayer::GetSkillEventParam(
            event,
            &mut horse_idx,
            &mut skill_id,
            &mut detail_index,
            &mut time_int,
            &mut target_flags,
            &mut caller_skill_id,
            &mut activate_type,
            &mut ability_value_status,
            &mut ability_time_status
        );

        if horse_idx < 0 || horse_idx as usize >= horse_count { continue; }
        let horse_info = horse_infos_slice[horse_idx as usize];
        if horse_info.is_null() { continue; }

        let skill_manager = HorseRaceInfo::get__skillManager(horse_info);
        if skill_manager.is_null() { continue; }
        SkillManager::AddUsedSkillId(skill_manager, skill_id);
    }
}

pub fn playback_gated(race_manager: *mut Il2CppObject) -> bool {
    HorseRaceInfo::is_start_dash() ||
    get_IsPlayingCutIn(race_manager) ||
    HorseRaceInfo::is_start_dash_instance(race_manager)
}

pub fn toggle_playback() {
    if !RaceHorseManagerBase::is_race_active() {
        return;
    }

    let race_manager = RaceManager::instance();
    if race_manager.is_null() { return; }

    if playback_gated(race_manager) { return; }

    if IsPaused(race_manager) {
        ResumeRace(race_manager);
    } else {
        PauseRace(race_manager);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceManagerReplayBase);

    unsafe {
        FORCE_SET_RACE_TIME_ADDR = get_method_addr(RaceManagerReplayBase, c"ForceSetRaceTime", 2);
        IS_PAUSED_ADDR = get_method_addr(RaceManagerReplayBase, c"IsPaused", 0);
        PAUSE_RACE_ADDR = get_method_addr(RaceManagerReplayBase, c"PauseRace", 0);
        RESUME_RACE_ADDR = get_method_addr(RaceManagerReplayBase, c"ResumeRace", 0);
        IS_PLAYING_CUT_IN_ADDR = get_method_addr(RaceManagerReplayBase, c"get_IsPlayingCutIn", 0);
        GET_JIKKYO_ADDR = get_method_addr(RaceManagerReplayBase, c"get_Jikkyo", 0);
        UPDATE_CAMERA_EVENT_ALL_ADDR = get_method_addr(RaceManagerReplayBase, c"UpdateCameraEventAll", 1);
        LATE_UPDATE_CAMERA_EVENT_ALL_ADDR = get_method_addr(RaceManagerReplayBase, c"LateUpdateCameraEventAll", 1);
        IS_CUT_IN_PLAYING_OR_RESERVED_ADDR = get_method_addr(RaceManagerReplayBase, c"IsCutInPlayingOrReserved", 0);
        UPDATE_HORSES_ADDR = get_method_addr(RaceManagerReplayBase, c"UpdateHorses", 1);
        UPDATE_HORSE_DATAS_ADDR = get_method_addr(RaceManagerReplayBase, c"UpdateHorseDatas", 1);
        UPDATE_HORSE_MODELS_ADDR = get_method_addr(RaceManagerReplayBase, c"UpdateHorseModels", 0);
        EVENT_PLAYER_FIELD = get_field_from_name(RaceManagerReplayBase, c"_eventPlayer");
    }
}

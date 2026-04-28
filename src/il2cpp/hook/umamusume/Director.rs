use crate::{
    core::{Hachimi, gui::IS_LIVE_SCENE},
    il2cpp::{
        ext::{Il2CppObjectExt, StringExt},
        symbols::{get_method_addr, Array, IList},
        types::*
    }
};
use std::sync::atomic::Ordering;

type AwakeFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Awake(this: *mut Il2CppObject) {
    IS_LIVE_SCENE.store(true, Ordering::Release);
    get_orig_fn!(Awake, AwakeFn)(this);

    let config = Hachimi::instance().config.load();
    if !config.champions_live_show_text { return; }

    unsafe {
        let get_load_settings_addr = crate::il2cpp::symbols::get_method_addr_cached((*this).klass(), c"get_LoadSettings", 0);
        if get_load_settings_addr == 0 { return; }

        let get_load_settings: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = std::mem::transmute(get_load_settings_addr);
        let load_settings = get_load_settings(this);
        if load_settings.is_null() { return; }

        let get_music_id_addr = crate::il2cpp::symbols::get_method_addr_cached((*load_settings).klass(), c"get_MusicId", 0);
        let get_music_id: extern "C" fn(*mut Il2CppObject) -> i32 = std::mem::transmute(get_music_id_addr);

        if get_music_id(load_settings) == 1054 {
            let get_race_info_addr = crate::il2cpp::symbols::get_method_addr_cached((*load_settings).klass(), c"get_raceInfo", 0);
            let get_race_info: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = std::mem::transmute(get_race_info_addr);
            let race_info = get_race_info(load_settings);

            if !race_info.is_null() {
                let get_cm_res_id_addr = crate::il2cpp::symbols::get_method_addr_cached((*race_info).klass(), c"get_ChampionsMeetingResourceId", 0);
                let get_cm_res_id: extern "C" fn(*mut Il2CppObject) -> i32 = std::mem::transmute(get_cm_res_id_addr);

                if get_cm_res_id(race_info) == 0 {
                    let set_cm_res_id_addr = crate::il2cpp::symbols::get_method_addr_cached((*race_info).klass(), c"set_ChampionsMeetingResourceId", 1);
                    let set_cm_res_id: extern "C" fn(*mut Il2CppObject, i32) = std::mem::transmute(set_cm_res_id_addr);
                    set_cm_res_id(race_info, config.champions_live_resource_id);

                    let set_date_year_addr = crate::il2cpp::symbols::get_method_addr_cached((*race_info).klass(), c"set_DateYear", 1);
                    let set_date_year: extern "C" fn(*mut Il2CppObject, i32) = std::mem::transmute(set_date_year_addr);
                    set_date_year(race_info, config.champions_live_year);

                    let mscorlib = crate::il2cpp::symbols::get_assembly_image(c"mscorlib.dll").unwrap();
                    let string_class = crate::il2cpp::symbols::get_class(mscorlib, c"System", c"String").unwrap();
                    let chara_name_array = Array::<*mut Il2CppString>::new(string_class, 9);
                    let trainer_name_array = Array::<*mut Il2CppString>::new(string_class, 9);

                    let get_chara_info_list_addr = crate::il2cpp::symbols::get_method_addr_cached((*load_settings).klass(), c"get_CharacterInfoList", 0);
                    let get_chara_info_list: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = std::mem::transmute(get_chara_info_list_addr);
                    let chara_info_list = get_chara_info_list(load_settings);

                    if let Some(ilist) = IList::<*mut Il2CppObject>::new(chara_info_list) {
                        for i in 0..9 {
                            let mut chara_name = "".to_il2cpp_string();
                            let trainer_name = "".to_il2cpp_string();

                            if let Some(info) = ilist.get(i as i32) {
                                let get_chara_id_addr = crate::il2cpp::symbols::get_method_addr_cached((*info).klass(), c"get_CharaId", 0);
                                let get_mob_id_addr = crate::il2cpp::symbols::get_method_addr_cached((*info).klass(), c"get_MobId", 0);

                                let get_chara_id: extern "C" fn(*mut Il2CppObject) -> i32 = std::mem::transmute(get_chara_id_addr);
                                let get_mob_id: extern "C" fn(*mut Il2CppObject) -> i32 = std::mem::transmute(get_mob_id_addr);

                                let chara_id = get_chara_id(info);
                                let mob_id = get_mob_id(info);

                                let name_str = if chara_id == 1 {
                                    crate::il2cpp::sql::get_master_text(59, mob_id).unwrap_or_else(|| "???".to_string())
                                } else {
                                    Hachimi::instance().chara_data.load().get_name(chara_id)
                                };
                                chara_name = name_str.to_il2cpp_string();
                            }

                            chara_name_array.as_slice()[i as usize] = chara_name;
                            trainer_name_array.as_slice()[i as usize] = trainer_name;
                        }
                    }

                    let set_chara_names_addr = crate::il2cpp::symbols::get_method_addr_cached((*race_info).klass(), c"set_CharacterNameArray", 1);
                    let set_trainer_names_addr = crate::il2cpp::symbols::get_method_addr_cached((*race_info).klass(), c"set_TrainerNameArray", 1);
                    let set_chara_names_text_addr = crate::il2cpp::symbols::get_method_addr_cached((*race_info).klass(), c"set_CharacterNameArrayForChampionsText", 1);
                    let set_trainer_names_text_addr = crate::il2cpp::symbols::get_method_addr_cached((*race_info).klass(), c"set_TrainerNameArrayForChampionsText", 1);

                    let set_chara_names: extern "C" fn(*mut Il2CppObject, *mut Il2CppArray) = std::mem::transmute(set_chara_names_addr);
                    let set_trainer_names: extern "C" fn(*mut Il2CppObject, *mut Il2CppArray) = std::mem::transmute(set_trainer_names_addr);
                    let set_chara_names_text: extern "C" fn(*mut Il2CppObject, *mut Il2CppArray) = std::mem::transmute(set_chara_names_text_addr);
                    let set_trainer_names_text: extern "C" fn(*mut Il2CppObject, *mut Il2CppArray) = std::mem::transmute(set_trainer_names_text_addr);

                    set_chara_names(race_info, chara_name_array.into());
                    set_trainer_names(race_info, trainer_name_array.into());
                    set_chara_names_text(race_info, std::ptr::null_mut());
                    set_trainer_names_text(race_info, std::ptr::null_mut());
                }
            }
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live", Director);
    let awake_addr = get_method_addr(Director, c"Awake", 0);
    new_hook!(awake_addr, Awake);
}
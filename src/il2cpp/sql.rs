use std::{ptr, sync::{atomic::{AtomicBool, Ordering}, Mutex, RwLock}};
use fnv::{FnvHashMap, FnvHashSet};
use sqlparser::ast;
use once_cell::sync::Lazy;
use crate::{
    core::{utils::{get_masterdb_path, get_meta_path}, Hachimi, game::Region},
    il2cpp::{ext::{StringExt, Il2CppStringExt}, hook::{LibNative_Runtime::Sqlite3::{Connection, Query}, umamusume::SceneManager}, types::{Il2CppObject, Il2CppString}}
};
use chrono::{Utc, Datelike};
use rust_i18n::locale;

pub static RETRIEVED_RAW_KEY: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static AUTO_UNLOCK_NEXT_DB: AtomicBool = AtomicBool::new(false);
pub static META_DATA: Lazy<RwLock<MetaData>> = Lazy::new(|| RwLock::new(MetaData::default()));

// public API
#[derive(Default)]
pub struct CharacterData {
    pub chara_ids: FnvHashSet<i32>,
    pub chara_names: FnvHashMap<i32, String>
}

impl CharacterData {
    pub fn load_from_db() -> Self {
        let mut chara_ids = FnvHashSet::default();
        let mut chara_names = FnvHashMap::default();

        let db_path = get_masterdb_path();
        let conn = Connection::new();

        if Connection::Open(conn, db_path.to_il2cpp_string(), ptr::null_mut(), ptr::null_mut(), 0) {
            let sql = "SELECT C.id, T.text FROM chara_data AS C JOIN text_data AS T ON C.id = T.\"index\" WHERE T.id = 6";
            let query = Connection::Query(conn, sql.to_il2cpp_string());

            if !query.is_null() {
                while Query::Step(query) {
                    let id = Query::GetInt(query, 0);
                    let name_ptr = Query::GetText(query, 1);

                    if let Some(name) = unsafe { name_ptr.as_ref() }.map(|s| s.as_utf16str().to_string()) {
                        chara_ids.insert(id);
                        chara_names.insert(id, name);
                    }
                }
                Query::Dispose(query);
            }
            Connection::CloseDB(conn);
        }

        CharacterData { chara_ids, chara_names }
    }

    pub fn exists(&self, id: i32) -> bool {
        self.chara_ids.contains(&id)
    }

    pub fn get_name(&self, id: i32) -> String {
        // check text_data_dict.json (category 170)
        if let Some(category_170) = Hachimi::instance().localized_data.load().text_data_dict.get(&170) {
            if let Some(name) = category_170.get(&id) {
                return name.clone();
            }
        }

        // fallback to default Japanese name from mdb
        if let Some(name) = self.chara_names.get(&id) {
            return name.clone();
        }

        // unknown character name
        "???".to_string()
    }
}

// untranslated skill info
#[derive(Default)]
pub struct SkillInfo {
    pub skill_names: FnvHashMap<i32, String>,
    pub skill_descs: FnvHashMap<i32, String>,
}

impl SkillInfo {
    pub fn load_from_db() -> Self {
        let mut skill_names = FnvHashMap::default();
        let mut skill_descs = FnvHashMap::default();

        let db_path = get_masterdb_path();
        let conn = Connection::new();

        if Connection::Open(conn, db_path.to_il2cpp_string(), ptr::null_mut(), ptr::null_mut(), 0) {
            // category 47 = names, 48 = descriptions
            let sql = "SELECT \"index\", text, id FROM text_data WHERE id IN (47, 48)";
            let query = Connection::Query(conn, sql.to_il2cpp_string());

            if !query.is_null() {
                while Query::Step(query) {
                    let index = Query::GetInt(query, 0);
                    let text_ptr = Query::GetText(query, 1);
                    let category = Query::GetInt(query, 2);

                    if let Some(text) = unsafe { text_ptr.as_ref() }.map(|s| s.as_utf16str().to_string()) {
                        match category {
                            47 => skill_names.insert(index, text),
                            48 => skill_descs.insert(index, text),
                            _ => None,
                        };
                    }
                }
                Query::Dispose(query);
            }
            Connection::CloseDB(conn);
        }

        SkillInfo { skill_names, skill_descs }
    }

    pub fn get_name(&self, id: i32) -> String {
        if let Some(name) = self.skill_names.get(&id) {
            return name.clone();
        }

        // unknown skill name
        "???".to_string()
    }

    pub fn get_desc(&self, id: i32) -> String {
        if let Some(desc) = self.skill_descs.get(&id) {
            return desc.clone();
        }

        // unknown skill desc
        "???".to_string()
    }
}

// All of this add column/param stuff could be simplified to two hash maps, but that's overkill.
pub trait SelectQueryState {
    /// Adds a column to the query.
    ///
    /// Implementers are expected to only track the index of columns that they need.
    fn add_column(&mut self, idx: i32, name: &str);

    /// Adds a placeholder parameter to the query (WHERE param = ?).
    ///
    /// Index starts at 1.
    fn add_param(&mut self, idx: i32, name: &str);

    /// Bind an int value to a placeholder.
    ///
    /// Index starts at 1.
    fn bind_int(&mut self, idx: i32, value: i32);

    /// Gets the resulting string on the current row's column.
    fn get_text(&self, query: *mut Il2CppObject, idx: i32) -> Option<*mut Il2CppString>;
}

#[derive(Default)]
struct Column {
    /// Index of the column in the SELECT statement.
    ///
    /// Can be used to query the value later if needed.
    select_idx: Option<i32>,

    /// Index of the placeholder param for this column.
    ///
    /// If this column's value is already binded as a param in the query, we won't need to query it later.
    param_idx: Option<i32>,

    /// The int value binded to this column as a parameter.
    int_value: Option<i32>
}

impl Column {
    fn is_select_idx(&self, idx: i32) -> bool {
        if let Some(i) = self.select_idx {
            idx == i
        }
        else {
            false
        }
    }

    fn is_param_idx(&self, idx: i32) -> bool {
        if let Some(i) = self.param_idx {
            idx == i
        }
        else {
            false
        }
    }

    fn try_bind_int(&mut self, idx: i32, value: i32) {
        if self.is_param_idx(idx) {
            self.int_value = Some(value);
        }
    }

    fn try_get_int(&self, query: *mut Il2CppObject) -> Option<i32> {
        if let Some(idx) = self.select_idx {
            Some(Query::GetInt(query, idx))
        }
        else {
            None
        }
    }

    fn value_or_try_get_int(&self, query: *mut Il2CppObject) -> Option<i32> {
        if let Some(value) = self.int_value {
            Some(value)
        }
        else if let Some(value) = self.try_get_int(query) {
            Some(value)
        }
        else {
            None
        }
    }
}

#[derive(Default)]
pub struct SkillDataDesc {
    pub descs: FnvHashMap<i32, String>
}

struct SkillDataDescRow {
    id: i32,
    precondition_1: String,
    condition_1: String,
    ability_time_1: i32,
    cooldown_time_1: i32,
    precondition_2: String,
    condition_2: String,
    ability_time_2: i32,
    cooldown_time_2: i32,
    slots: [SkillDataDescSlot; 6]
}

#[derive(Clone, Copy, Default)]
struct SkillDataDescSlot {
    ability_type: i32,
    ability_value: i32,
    ability_value_usage: i32,
    additional_activate_type: i32,
    target_type: i32,
    target_value: i32
}

impl SkillDataDesc {
    pub fn load_from_db() -> Self {
        let mut descs = FnvHashMap::default();

        let db_path = get_masterdb_path();
        let conn = Connection::new();

        if Connection::Open(conn, db_path.to_il2cpp_string(), ptr::null_mut(), ptr::null_mut(), 0) {
            let sql = "SELECT id, \
                precondition_1, condition_1, float_ability_time_1, float_cooldown_time_1, \
                ability_type_1_1, ability_value_usage_1_1, additional_activate_type_1_1, float_ability_value_1_1, target_type_1_1, target_value_1_1, \
                ability_type_1_2, ability_value_usage_1_2, additional_activate_type_1_2, float_ability_value_1_2, target_type_1_2, target_value_1_2, \
                ability_type_1_3, ability_value_usage_1_3, additional_activate_type_1_3, float_ability_value_1_3, target_type_1_3, target_value_1_3, \
                precondition_2, condition_2, float_ability_time_2, float_cooldown_time_2, \
                ability_type_2_1, ability_value_usage_2_1, additional_activate_type_2_1, float_ability_value_2_1, target_type_2_1, target_value_2_1, \
                ability_type_2_2, ability_value_usage_2_2, additional_activate_type_2_2, float_ability_value_2_2, target_type_2_2, target_value_2_2, \
                ability_type_2_3, ability_value_usage_2_3, additional_activate_type_2_3, float_ability_value_2_3, target_type_2_3, target_value_2_3 \
                FROM skill_data";
            let query = Connection::Query(conn, sql.to_il2cpp_string());

            if !query.is_null() {
                while Query::Step(query) {
                    let row = Self::get_data_row(query);
                    let desc = Self::format_data_desc(&row);
                    descs.insert(row.id, desc);
                }
                Query::Dispose(query);
            }
            Connection::CloseDB(conn);
        }

        SkillDataDesc { descs }
    }

    pub fn get_desc(&self, id: i32) -> Option<&String> {
        self.descs.get(&id)
    }
    
    fn get_data_slot(query: *mut Il2CppObject, base: i32) -> SkillDataDescSlot {
        SkillDataDescSlot {
            ability_type: Query::GetInt(query, base),
            ability_value_usage: Query::GetInt(query, base + 1),
            additional_activate_type: Query::GetInt(query, base + 2),
            ability_value: Query::GetInt(query, base + 3),
            target_type: Query::GetInt(query, base + 4),
            target_value: Query::GetInt(query, base + 5)
        }
    }

    fn get_data_text(query: *mut Il2CppObject, idx: i32) -> String {
        let text_ptr = Query::GetText(query, idx);
        unsafe { text_ptr.as_ref() }.map(|s| s.as_utf16str().to_string()).unwrap_or_default()
    }

    fn get_data_row(query: *mut Il2CppObject) -> SkillDataDescRow {
        SkillDataDescRow {
            id: Query::GetInt(query, 0),
            precondition_1: Self::get_data_text(query, 1),
            condition_1: Self::get_data_text(query, 2),
            ability_time_1: Query::GetInt(query, 3),
            cooldown_time_1: Query::GetInt(query, 4),
            precondition_2: Self::get_data_text(query, 23),
            condition_2: Self::get_data_text(query, 24),
            ability_time_2: Query::GetInt(query, 25),
            cooldown_time_2: Query::GetInt(query, 26),
            slots: [
                Self::get_data_slot(query, 5), Self::get_data_slot(query, 11), Self::get_data_slot(query, 17),
                Self::get_data_slot(query, 27), Self::get_data_slot(query, 33), Self::get_data_slot(query, 39)
            ]
        }
    }

    fn round_ties_up(value: i32, units: i32) -> i32 {
        let rem = value.rem_euclid(units);
        let base = value - rem;
        if rem * 2 >= units { base + units } else { base }
    }

    fn format_data_number(value: i32, div: i32, decimals: usize) -> String {
        let units = div / 10i32.pow(decimals as u32);
        let rounded = Self::round_ties_up(value, units);
        let neg = rounded < 0;
        let abs = rounded.unsigned_abs() as u64;
        let div = div as u64;
        let whole = abs / div;
        let frac = (abs % div) / (div / 10u64.pow(decimals as u32));

        let mut out = String::new();
        if neg {
            out.push('-');
        }
        out.push_str(&whole.to_string());
        if frac > 0 {
            out.push('.');
            let frac_str = format!("{:0width$}", frac, width = decimals);
            out.push_str(frac_str.trim_end_matches('0'));
        }
        out
    }

    fn str(key: &str) -> Option<String> {
        let full_key = format!("skill_data_desc.{key}");
        let localized_data = Hachimi::instance().localized_data.load();
        if let Some(text) = localized_data.skill_data_desc_dict.get(full_key.as_str()) {
            return Some(text.to_string());
        }
        let locale = locale();
        crate::_rust_i18n_try_translate(&locale, full_key.as_str()).map(|text| text.to_string())
    }

    fn data_fmt(key: &str, value: &str) -> Option<String> {
        Self::str(key).map(|text| text.replace("%{v}", value))
    }

    fn op_tag(op: &str) -> &str {
        match op {
            "==" => "eq",
            "!=" => "ne",
            "<=" => "le",
            ">=" => "ge",
            "<" => "lt",
            ">" => "gt",
            _ => "op"
        }
    }

    fn format_effect(slot: SkillDataDescSlot) -> Option<String> {
        let (name_key, unit_key, div, decimals) = match slot.ability_type {
            1 => ("speed_stat", "stat", 10000, 2),
            2 => ("stamina_stat", "stat", 10000, 2),
            3 => ("power_stat", "stat", 10000, 2),
            4 => ("guts_stat", "stat", 10000, 2),
            5 => ("wit_stat", "stat", 10000, 2),
            8 => ("field_of_view", "deg", 10000, 2),
            9 => ("current_hp", "percent", 100, 1),
            13 => ("rushed_time", "second", 10000, 2),
            14 => ("delay_start", "second", 10000, 2),
            21 => ("current_speed", "mps", 10000, 2),
            22 => ("current_speed_natural_decel", "mps", 10000, 2),
            27 => ("target_speed", "mps", 10000, 2),
            28 => ("lane_movement_speed", "percent", 100, 1),
            29 => ("rushed_chance", "stat", 10000, 2),
            31 => ("acceleration", "mps2", 10000, 2),
            32 => ("all_stats", "stat", 10000, 2),
            35 => ("target_lane", "stat", 10000, 2),
            37 => ("activate_rare_skill", "stat", 10000, 2),
            42 | 48 | 49 => ("special", "stat", 10000, 2),
            501 => ("event_specific", "stat", 10000, 2),

            6 => return Self::str("effect.fixed.aggressive_strategy"),
            38 => return Self::str("effect.fixed.debuff_immunity"),
            41 => return Self::str("effect.fixed.sympathy_all"),
            502 => return Self::str("effect.fixed.loh_stat"),
            10 => return Self::str(&format!("effect.start_reaction.{}", slot.ability_value)),
            503 | _ => return None,
        };

        let name = Self::str(&format!("effect.name.{name_key}"))?;
        let unit = Self::str(&format!("effect.unit.{unit_key}")).unwrap_or_default();
        let value = Self::format_data_number(slot.ability_value, div, decimals);
        let sign = if slot.ability_value > 0 { " +" } else { " " };
        let mut out = format!("{name}{sign}{value}{unit}");
        if slot.ability_value_usage == 19 {
            out.push_str(&Self::str("effect.usage19_suffix").unwrap_or_default());
        }

        let star = match slot.additional_activate_type {
            1 => Self::str("star.activate.1"),
            2 => Self::str("star.activate.2"),
            3 => Self::str("star.activate.3"),
            _ => None
        }.or_else(|| {
            if slot.ability_value_usage != 1 {
                Self::str(&format!("star.usage.{}", slot.ability_value_usage))
            } else {
                None
            }
        });
        if let Some(star) = star {
            out.push_str(&Self::str("sep.star").unwrap_or_default());
            out.push_str(&star);
        }

        if slot.target_type != 1 {
            let target = match slot.target_type {
                4 => Self::str("target.all_in_fov"),
                7 => Self::data_fmt("target.leading", &(slot.target_value - 1).to_string()),
                9 => if slot.target_value == 18 {
                    Self::str("target.all_ahead")
                } else {
                    Self::data_fmt("target.closest_ahead", &slot.target_value.to_string())
                },
                10 => if slot.target_value == 18 {
                    Self::str("target.all_behind")
                } else {
                    Self::data_fmt("target.closest_behind", &slot.target_value.to_string())
                },
                11 => Self::str("target.team"),
                18 => match Self::str(&format!("target.style.{}", slot.target_value)) {
                    Some(text) => Some(text),
                    None => return None
                },
                19 => Self::data_fmt("target.random_rushed_ahead", &slot.target_value.to_string()),
                20 => Self::data_fmt("target.random_rushed_behind", &slot.target_value.to_string()),
                21 => match Self::str(&format!("target.style_rushed.{}", slot.target_value)) {
                    Some(text) => Some(text),
                    None => return None
                },
                22 => Self::str("target.suzuka"),
                23 => Self::data_fmt("target.random_recovery_users", &slot.target_value.to_string()),
                24 => Self::str("target.unknown"),
                _ => None
            };
            if let Some(target) = target {
                out.push_str(&Self::str("sep.to").unwrap_or_default());
                out.push_str(&target);
            }
        }

        Some(out)
    }

    fn format_data_conditions(condition: &str) -> String {
        let or_sep = Self::str("sep.or").unwrap_or_default();
        let and_sep = Self::str("sep.and").unwrap_or_default();
        let mut out = String::new();
        for (i, group) in condition.split('@').enumerate() {
            if i > 0 {
                out.push_str(&or_sep);
            }
            for (j, atom) in group.split('&').enumerate() {
                if j > 0 {
                    out.push_str(&and_sep);
                }
                out.push_str(&Self::format_data_atom(atom));
            }
        }
        out
    }

    fn format_data_atom(atom: &str) -> String {
        let bytes = atom.as_bytes();
        let mut token_end = 0;
        while token_end < bytes.len() && (bytes[token_end].is_ascii_lowercase() || bytes[token_end] == b'_' || bytes[token_end].is_ascii_digit()) {
            token_end += 1;
        }
        let op_start = token_end;
        let mut op_end = op_start;
        while op_end < bytes.len() && (bytes[op_end] == b'=' || bytes[op_end] == b'!' || bytes[op_end] == b'<' || bytes[op_end] == b'>') {
            op_end += 1;
        }
        let token = &atom[..token_end];
        let op = &atom[op_start..op_end];
        let value = atom[op_end..].parse::<i32>().unwrap_or(0);

        if token == "order_rate" {
            let text = match op {
                ">" => Self::data_fmt("cond.order_rate.gt", &(100 - value).to_string()),
                ">=" => Self::data_fmt("cond.order_rate.ge", &(100 - value).to_string()),
                "<=" => Self::data_fmt("cond.order_rate.le", &value.to_string()),
                "<" => Self::data_fmt("cond.order_rate.lt", &value.to_string()),
                _ => None
            };
            if let Some(text) = text {
                return text;
            }
        }

        if token == "corner" {
            let text = match (op, value) {
                ("==", 0) => Self::str("cond.corner.straight"),
                ("==", _) => Self::data_fmt("cond.corner.corner", &value.to_string()),
                ("!=", 0) => Self::str("cond.corner.any"),
                ("!=", _) => Self::data_fmt("cond.corner.not", &value.to_string()),
                _ => None
            };
            if let Some(text) = text {
                return text;
            }
        }

        if token == "phase" && matches!(op, "==" | "!=" | "<=" | ">=") {
            if let Some(name) = Self::str(&format!("cond.phase.name.{value}")) {
                return match op {
                    "==" => name,
                    "!=" => Self::data_fmt("cond.negate", &name).unwrap_or_default(),
                    "<=" => Self::data_fmt("cond.phase.le", &name).unwrap_or_default(),
                    _ => Self::data_fmt("cond.phase.ge", &name).unwrap_or_default()
                };
            }
        }

        if token == "ground_condition" && matches!(op, "==" | "!=" | "<=" | ">=") {
            if let Some(name) = Self::str(&format!("cond.ground_condition.name.{value}")) {
                if let Some(text) = Self::data_fmt(&format!("cond.ground_condition.{}", Self::op_tag(op)), &name) {
                    return text;
                }
            }
        }

        if token == "track_id" {
            if op == "<=" && value == 10010 {
                if let Some(text) = Self::str("cond.track_id.jra") {
                    return text;
                }
            }
            if op == ">=" && value == 10001 {
                if let Some(text) = Self::str("cond.track_id.any") {
                    return text;
                }
            }
            if op == "==" || op == "!=" {
                if let Some(name) = Self::str(&format!("cond.track_name.{value}")) {
                    let key = if op == "==" { "cond.track_id.at" } else { "cond.track_id.not_at" };
                    if let Some(text) = Self::data_fmt(key, &name) {
                        return text;
                    }
                }
            }
        }

        if token == "same_skill_horse_count" && op == "==" {
            let text = if value == 1 {
                Self::str("cond.same_skill_horse_count.unique")
            } else {
                Self::data_fmt("cond.same_skill_horse_count.count", &value.to_string())
            };
            if let Some(text) = text {
                return text;
            }
        }

        if token == "near_infront_count" && op == "==" {
            let text = if value == 0 {
                Self::str("cond.near_infront_count.none")
            } else {
                Self::data_fmt("cond.near_infront_count.count", &value.to_string())
            };
            if let Some(text) = text {
                return text;
            }
        }

        if let Some(text) = Self::data_condition_enum(token, op, value) {
            return text;
        }

        if op == "!=" {
            if let Some(text) = Self::data_condition_enum(token, "==", value) {
                if let Some(negated) = Self::data_fmt("cond.negate", &text) {
                    return negated;
                }
            }
        }

        if let Some(text) = Self::data_condition_fixed(token, op) {
            return text;
        }

        if token == "distance_diff_top_float" && op == "<=" {
            if let Some(text) = Self::data_fmt("cond.template.distance_diff_top_float.le", &Self::format_data_number(value, 10, 1)) {
                return text;
            }
        }

        if let Some(text) = Self::data_condition_template(token, op, value) {
            return text;
        }

        if token == "furlong" && op == "==" {
            if let Some(text) = Self::data_fmt("cond.furlong", &(value + 1).to_string()) {
                return text;
            }
        }

        if token == "is_used_skill_id" && op == "==" {
            if let Some(text) = Self::str(&format!("cond.used_skill.{value}")) {
                return text;
            }
            if let Some(text) = Self::data_fmt("cond.used_skill.template", &value.to_string()) {
                return text;
            }
        }

        if token == "is_used_skill_id_with_detail_one" && op == "==" {
            if let Some(text) = Self::str(&format!("cond.used_skill_detail_one.{value}")) {
                return text;
            }
            if let Some(text) = Self::data_fmt("cond.used_skill_detail_one.template", &value.to_string()) {
                return text;
            }
        }

        if token == "is_popularity_top_character_activate_advantage_skill" && op == "==" {
            if value == -1 {
                if let Some(text) = Self::str("cond.popularity_top.any") {
                    return text;
                }
            }
            if let Some(text) = Self::data_fmt("cond.popularity_top.count", &value.to_string()) {
                return text;
            }
        }

        format!("{token} {op} {value}")
    }

    fn data_condition_enum(token: &str, op: &str, value: i32) -> Option<String> {
        Self::str(&format!("cond.enum.{token}.{}.{}", Self::op_tag(op), value))
    }

    fn data_condition_fixed(token: &str, op: &str) -> Option<String> {
        Self::str(&format!("cond.fixed.{token}.{}", Self::op_tag(op)))
    }

    fn data_condition_template(token: &str, op: &str, value: i32) -> Option<String> {
        Self::str(&format!("cond.template.{token}.{}", Self::op_tag(op)))
            .map(|text| text.replace("%{v}", &value.to_string()))
    }

    fn format_data_group(condition: &str, precondition: &str, ability_time: i32, cooldown_time: i32, slots: &[SkillDataDescSlot]) -> Option<String> {
        let mut effects: Vec<String> = Vec::new();
        for slot in slots {
            if slot.ability_type == 0 && slot.ability_value == 0 {
                continue;
            }
            if let Some(effect) = Self::format_effect(*slot) {
                effects.push(effect);
            }
        }
        if effects.is_empty() {
            return None;
        }

        let first_type = slots.first().map(|s| s.ability_type).unwrap_or(0);
        let first_value = slots.first().map(|s| s.ability_value).unwrap_or(0);
        let time_suffix = if ability_time > 0 {
            Self::data_fmt("group.duration", &Self::format_data_number(ability_time, 10000, 2))
        } else if ability_time == 0 {
            Self::str("group.immediate")
        } else if first_type == 21 && first_value < 0 {
            Self::str("group.long_negative")
        } else {
            Self::str("group.indefinite")
        }.unwrap_or_default();

        let mut body = effects.join(", ");
        body.push(' ');
        body.push_str(&time_suffix);

        let mut line = format!("<b>{body}</b>");
        if cooldown_time > 0 && cooldown_time < 5000000 {
            line.push_str(&Self::data_fmt("group.cd", &format!("{:.1}", cooldown_time as f64 / 10000.0)).unwrap_or_default());
        }
        line.push_str(&Self::str("group.when").unwrap_or_default());
        line.push_str(&Self::format_data_conditions(if condition.is_empty() { "always==1" } else { condition }));
        if !precondition.is_empty() {
            line.push_str(&Self::str("group.after").unwrap_or_default());
            line.push_str(&Self::format_data_conditions(precondition));
        }
        Some(line)
    }

    fn format_data_desc(row: &SkillDataDescRow) -> String {
        let group1 = Self::format_data_group(&row.condition_1, &row.precondition_1, row.ability_time_1, row.cooldown_time_1, &row.slots[0..3]);
        let group2 = Self::format_data_group(&row.condition_2, &row.precondition_2, row.ability_time_2, row.cooldown_time_2, &row.slots[3..6]);

        match (group1, group2) {
            (Some(g1), Some(g2)) => format!("{g1}\n{g2}"),
            (Some(g1), None) => g1,
            (None, Some(g2)) => g2,
            (None, None) => String::new()
        }
    }
}

// text_data
#[derive(Default)]
pub struct TextDataQuery {
    // SELECT
    text: Column,

    // WHERE
    category: Column,
    index: Column
}

impl TextDataQuery {
    pub fn get_skill_name(index: i32) -> Option<*mut Il2CppString> {
        // Return None if skill name translation is disabled
        if Hachimi::instance().config.load().disable_skill_name_translation {
            return None;
        }

        let localized_data = Hachimi::instance().localized_data.load();
        localized_data.text_data_dict
            .get(&47)
            .and_then(|c| c.get(&index))
            .map(|t| t.to_il2cpp_string())
    }

    pub fn get_skill_desc(index: i32) -> Option<*mut Il2CppString> {
        if Hachimi::instance().config.load().skill_data_desc {
            let skill_data_desc = Hachimi::instance().skill_data_desc.load();
            if let Some(desc) = skill_data_desc.get_desc(index) {
                return Some(desc.to_il2cpp_string());
            }
        }

        let localized_data = Hachimi::instance().localized_data.load();
        localized_data
            .text_data_dict
            .get(&48)
            .and_then(|c| c.get(&index))
            .map(|t| t.to_il2cpp_string())
    }
}

impl SelectQueryState for TextDataQuery {
    fn add_column(&mut self, idx: i32, name: &str) {
        if name == "text" {
            self.text.select_idx = Some(idx)
        }
    }

    fn add_param(&mut self, idx: i32, name: &str) {
        match name {
            "category" => self.category.param_idx = Some(idx),
            "index" => self.index.param_idx = Some(idx),
            _ => ()
        }
    }

    fn bind_int(&mut self, idx: i32, value: i32) {
        self.category.try_bind_int(idx, value);
        self.index.try_bind_int(idx, value);
    }

    fn get_text(&self, _query: *mut Il2CppObject, idx: i32) -> Option<*mut Il2CppString> {
        if !self.text.is_select_idx(idx) {
            return None;
        }

        if let Some(category) = self.category.int_value {
            if let Some(index) = self.index.int_value {
                // specialized handlers
                match category {
                    47 => return Self::get_skill_name(index),
                    48 => return Self::get_skill_desc(index),
                    _ => ()
                };

                return Hachimi::instance().localized_data.load()
                    .text_data_dict
                    .get(&category)
                    .map(|c| c.get(&index).map(|s| s.to_il2cpp_string()))
                    .unwrap_or_default()
            }
        }

        None
    }
}

// character_system_text
#[derive(Default)]
pub struct CharacterSystemTextQuery {
    // SELECT
    text: Column,

    // WHERE
    character_id: Column,

    // may appear in both
    voice_id: Column
}

impl SelectQueryState for CharacterSystemTextQuery {
    fn add_column(&mut self, idx: i32, name: &str) {
        match name {
            "text" => self.text.select_idx = Some(idx),
            "voice_id" => self.voice_id.select_idx = Some(idx),
            _ => ()
        }
    }

    fn add_param(&mut self, idx: i32, name: &str) {
        match name {
            "character_id" => self.character_id.param_idx = Some(idx),
            "voice_id" => self.voice_id.param_idx = Some(idx),
            _ => ()
        }
    }

    fn bind_int(&mut self, idx: i32, value: i32) {
        self.character_id.try_bind_int(idx, value);
        self.voice_id.try_bind_int(idx, value);
    }

    fn get_text(&self, query: *mut Il2CppObject, idx: i32) -> Option<*mut Il2CppString> {
        if !self.text.is_select_idx(idx) {
            return None;
        }

        if let Some(character_id) = self.character_id.int_value {
            if let Some(voice_id) = self.voice_id.value_or_try_get_int(query) {
                return Hachimi::instance().localized_data.load()
                    .character_system_text_dict
                    .get(&character_id)
                    .map(|c| c.get(&voice_id).map(|s| s.to_il2cpp_string()))
                    .unwrap_or_default()
            }
        }

        None
    }
}

// race_jikkyo_comment
#[derive(Default)]
pub struct RaceJikkyoCommentQuery {
    // SELECT
    id: Column,
    message: Column
}

impl SelectQueryState for RaceJikkyoCommentQuery {
    fn add_column(&mut self, idx: i32, name: &str) {
        match name {
            "id" => self.id.select_idx = Some(idx),
            "message" => self.message.select_idx = Some(idx),
            _ => ()
        }
    }

    fn add_param(&mut self, _idx: i32, _name: &str) {}

    fn bind_int(&mut self, _idx: i32, _value: i32) {}

    fn get_text(&self, query: *mut Il2CppObject, idx: i32) -> Option<*mut Il2CppString> {
        if !self.message.is_select_idx(idx) {
            return None;
        }

        if let Some(id) = self.id.try_get_int(query) {
            return Hachimi::instance().localized_data.load()
                .race_jikkyo_comment_dict
                .get(&id)
                .map(|s| s.to_il2cpp_string())
        }

        None
    }
}

// race_jikkyo_message
#[derive(Default)]
pub struct RaceJikkyoMessageQuery {
    // SELECT
    id: Column,
    message: Column
}

impl SelectQueryState for RaceJikkyoMessageQuery {
    fn add_column(&mut self, idx: i32, name: &str) {
        match name {
            "id" => self.id.select_idx = Some(idx),
            "message" => self.message.select_idx = Some(idx),
            _ => ()
        }
    }

    fn add_param(&mut self, _idx: i32, _name: &str) {}

    fn bind_int(&mut self, _idx: i32, _value: i32) {}

    fn get_text(&self, query: *mut Il2CppObject, idx: i32) -> Option<*mut Il2CppString> {
        if !self.message.is_select_idx(idx) {
            return None;
        }

        if let Some(id) = self.id.try_get_int(query) {
            return Hachimi::instance().localized_data.load()
                .race_jikkyo_message_dict
                .get(&id)
                .map(|s| s.to_il2cpp_string())
        }

        None
    }
}


// sqlparser extensions
pub trait SelectExt {
    fn get_first_table_name(&self) -> Option<&String>;
}

impl SelectExt for ast::Select {
    fn get_first_table_name(&self) -> Option<&String> {
        if let Some(table_with_joins) = self.from.get(0) {
            if let ast::TableFactor::Table { name: object_name, .. } = &table_with_joins.relation {
                if let Some(ident) = object_name.0.get(0) {
                    return Some(&ident.value);
                }
            }
        }

        None
    }
}

pub trait SelectItemExt {
    fn get_unnamed_expr_ident(&self) -> Option<&String>;
}

impl SelectItemExt for ast::SelectItem {
    fn get_unnamed_expr_ident(&self) -> Option<&String> {
        if let ast::SelectItem::UnnamedExpr(expr) = self {
            return expr.get_ident_value();
        }

        None
    }
}

pub trait ExprExt {
    fn binary_op_iter<'a>(&'a self) -> BinaryOpIter<'a>;
    fn get_ident_value(&self) -> Option<&String>;
    fn is_placeholder_value(&self) -> bool;
}

impl ExprExt for ast::Expr {
    fn binary_op_iter<'a>(&'a self) -> BinaryOpIter<'a> {
        BinaryOpIter { stack: vec![self] }
    }

    fn get_ident_value(&self) -> Option<&String> {
        if let ast::Expr::Identifier(ident) = self {
            return Some(&ident.value);
        }

        None
    }

    fn is_placeholder_value(&self) -> bool {
        if let ast::Expr::Value(value) = self {
            if let ast::Value::Placeholder(_) = value {
                return true;
            }
        }

        false
    }
}

pub struct BinaryOpIter<'a> {
    stack: Vec<&'a ast::Expr>
}

pub struct BinaryOpRef<'a> {
    pub left: &'a Box<ast::Expr>,
    pub op: &'a ast::BinaryOperator,
    pub right: &'a Box<ast::Expr>
}

impl<'a> Iterator for BinaryOpIter<'a> {
    type Item = BinaryOpRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(expr) = self.stack.pop() else {
                return None;
            };

            let ast::Expr::BinaryOp { left, op, right } = expr else {
                continue;
            };

            self.stack.push(right);
            self.stack.push(left); // left will be pop'd first

            return Some(BinaryOpRef { left, op, right })
        }
    }
}

#[derive(Default)]
pub struct MetaData {
    pub logical_name_to_hash: FnvHashMap<String, String>,
}

impl MetaData {
    pub fn get_hash(logical_name: &str) -> Option<String> {
        {
            let meta_read = META_DATA.read().unwrap();
            if !meta_read.logical_name_to_hash.is_empty() {
                return meta_read.logical_name_to_hash.get(logical_name).cloned();
            }
        }

        let mut meta_write = META_DATA.write().unwrap();

        if meta_write.logical_name_to_hash.is_empty() {
            if RETRIEVED_RAW_KEY.lock().unwrap().is_empty() {
                return None;
            }
            let loaded = Self::load_from_db();
            meta_write.logical_name_to_hash = loaded.logical_name_to_hash;
        }

        meta_write.logical_name_to_hash.get(logical_name).cloned()
    }

    fn load_from_db() -> Self {
        let mut logical_name_to_hash = FnvHashMap::default();

        let db_path_str = get_meta_path();

        let conn = Connection::new();

        if Hachimi::instance().game.region == Region::Japan {
            AUTO_UNLOCK_NEXT_DB.store(true, Ordering::Relaxed);
        }

        if Connection::Open(conn, db_path_str.to_il2cpp_string(), std::ptr::null_mut(), std::ptr::null_mut(), 0) {
            let sql = "SELECT n, h FROM a";
            let query = Connection::Query(conn, sql.to_il2cpp_string());

            if !query.is_null() {
                while Query::Step(query) {
                    let path_ptr = Query::GetText(query, 0);
                    let hash_ptr = Query::GetText(query, 1);

                    if let (Some(path_str), Some(hash_str)) = (
                        unsafe { path_ptr.as_ref() }.map(|s| s.as_utf16str().to_string()),
                        unsafe { hash_ptr.as_ref() }.map(|s| s.as_utf16str().to_string()),
                    ) {
                        let logical_name = if let Some(idx) = path_str.rfind('/') {
                            format!("{}.a", &path_str[idx + 1..])
                        } else {
                            format!("{}.a", path_str)
                        };

                        logical_name_to_hash.insert(logical_name, hash_str);
                    }
                }
                Query::Dispose(query);
            }
            Connection::CloseDB(conn);
        } else {
            error!("Failed to open meta database at: {}", db_path_str);
        }

        MetaData { logical_name_to_hash }
    }
}

fn get_single_column_int(sql: &str) -> Vec<i32> {
    let mut items = Vec::new();
    let db_path = get_masterdb_path();
    let conn = Connection::new();
    if Connection::Open(conn, db_path.to_il2cpp_string(), std::ptr::null_mut(), std::ptr::null_mut(), 0) {
        let query = Connection::Query(conn, sql.to_il2cpp_string());
        if !query.is_null() {
            while Query::Step(query) {
                items.push(Query::GetInt(query, 0));
            }
            Query::Dispose(query);
        }
        Connection::CloseDB(conn);
    }
    items
}

pub fn get_all_chara_ids() -> Vec<i32> {
    get_single_column_int("SELECT id FROM chara_data")
}

pub fn get_all_dress_ids() -> Vec<i32> {
    get_single_column_int("SELECT id FROM dress_data")
}

pub fn get_all_music_ids() -> Vec<i32> {
    get_single_column_int("SELECT music_id FROM live_data")
}

pub fn get_all_mob_ids() -> Vec<i32> {
    get_single_column_int("SELECT mob_id FROM mob_data WHERE use_live = 1")
}

pub fn get_default_dress_ids() -> Vec<i32> {
    get_single_column_int("SELECT id FROM dress_data WHERE (condition_type = 1 OR condition_type = 4 OR condition_type = 5) AND use_live_theater = 1 AND id < 999")
}

pub fn get_all_cards() -> Vec<(i32, i32)> {
    let mut items = Vec::new();
    let db_path = get_masterdb_path();
    let conn = Connection::new();
    if Connection::Open(conn, db_path.to_il2cpp_string(), std::ptr::null_mut(), std::ptr::null_mut(), 0) {
        let query = Connection::Query(conn, "SELECT id, default_rarity FROM card_data WHERE id <= 999999".to_il2cpp_string());
        if !query.is_null() {
            while Query::Step(query) {
                items.push((Query::GetInt(query, 0), Query::GetInt(query, 1)));
            }
            Query::Dispose(query);
        }
        Connection::CloseDB(conn);
    }
    items
}

pub fn get_master_text(category: i32, index: i32) -> Option<String> {
    let db_path = get_masterdb_path();
    let conn = Connection::new();
    if Connection::Open(conn, db_path.to_il2cpp_string(), std::ptr::null_mut(), std::ptr::null_mut(), 0) {
        let sql = format!("SELECT text FROM text_data WHERE \"category\" = {} AND \"index\" = {}", category, index);
        let query = Connection::Query(conn, sql.to_il2cpp_string());
        if !query.is_null() {
            if Query::Step(query) {
                let text_ptr = Query::GetText(query, 0);
                if let Some(text) = unsafe { text_ptr.as_ref() }.map(|s| s.as_utf16str().to_string()) {
                    Query::Dispose(query);
                    Connection::CloseDB(conn);
                    return Some(text);
                }
            }
            Query::Dispose(query);
        }
        Connection::CloseDB(conn);
    }
    None
}

pub fn get_jobs_info(reward_id: i32) -> Option<(i32, i32)> {
    let db_path = get_masterdb_path();
    let conn = Connection::new();
    if Connection::Open(conn, db_path.to_il2cpp_string(), std::ptr::null_mut(), std::ptr::null_mut(), 0) {
        let sql = format!("SELECT place_id, genre_id FROM jobs_reward WHERE \"id\" = {}", reward_id);
        let query = Connection::Query(conn, sql.to_il2cpp_string());
        if !query.is_null() {
            if Query::Step(query) {
                let place_id = Query::GetInt(query, 0);
                let genre_id = Query::GetInt(query, 1);
                Query::Dispose(query);
                Connection::CloseDB(conn);
                return Some((place_id, genre_id));
            }
            Query::Dispose(query);
        }
        Connection::CloseDB(conn);
    }
    None
}

pub fn get_jobs_place_race_track_id(place_id: i32) -> Option<i32> {
    let db_path = get_masterdb_path();
    let conn = Connection::new();
    if Connection::Open(conn, db_path.to_il2cpp_string(), std::ptr::null_mut(), std::ptr::null_mut(), 0) {
        let sql = format!("SELECT race_track_id FROM jobs_place WHERE \"id\" = {}", place_id);
        let query = Connection::Query(conn, sql.to_il2cpp_string());
        if !query.is_null() {
            if Query::Step(query) {
                let track_id = Query::GetInt(query, 0);
                Query::Dispose(query);
                Connection::CloseDB(conn);
                return Some(track_id);
            }
            Query::Dispose(query);
        }
        Connection::CloseDB(conn);
    }
    None
}

pub fn get_champions_resources() -> Vec<String> {
    let mut items = Vec::new();
    let db_path = get_masterdb_path();
    let conn = Connection::new();
    if Connection::Open(conn, db_path.to_il2cpp_string(), ptr::null_mut(), ptr::null_mut(), 0) {
        let sql = "SELECT t.text FROM champions_schedule c LEFT OUTER JOIN text_data t on t.category = 206 AND t.\"index\" = c.id GROUP BY c.resource_id";
        let query = Connection::Query(conn, sql.to_il2cpp_string());
        if !query.is_null() {
            while Query::Step(query) {
                let text_ptr = Query::GetText(query, 0);
                if let Some(text) = unsafe { text_ptr.as_ref() }.map(|s| s.as_utf16str().to_string()) {
                    items.push(text);
                } else {
                    items.push(rust_i18n::t!("unknown").into_owned());
                }
            }
            Query::Dispose(query);
        }
        Connection::CloseDB(conn);
    }
    items
}

pub fn get_champions_live_max_year() -> i32 {
    let mut max_year = Utc::now().year(); // fallback to the current year since it's guaranteed to have textures
    if !SceneManager::is_home_init() { return max_year; }
    let db_path_str = get_meta_path();

    let conn = Connection::new();
    if Hachimi::instance().game.region == Region::Japan {
        AUTO_UNLOCK_NEXT_DB.store(true, Ordering::Relaxed);
    }
    if Connection::Open(conn, db_path_str.to_il2cpp_string(), ptr::null_mut(), ptr::null_mut(), 0) {
        let sql = "SELECT n FROM a WHERE n LIKE 'live/image/champions/tex_championslive_year_%'";
        let query = Connection::Query(conn, sql.to_il2cpp_string());

        if !query.is_null() {
            let mut max_idx = -1;
            while Query::Step(query) {
                let text_ptr = Query::GetText(query, 0);
                if let Some(text) = unsafe { text_ptr.as_ref() }.map(|s| s.as_utf16str().to_string()) {
                    if let Some(idx_str) = text.strip_prefix("live/image/champions/tex_championslive_year_") {
                        if let Ok(idx) = idx_str.parse::<i32>() {
                            max_idx = max_idx.max(idx);
                        }
                    }
                }
            }
            Query::Dispose(query);
            if max_idx >= 0 {
                max_year = 2022 + max_idx;
            }
        }
        Connection::CloseDB(conn);
    }
    max_year
}

use std::{borrow::Cow, fs::File, io::Write, sync::Mutex, path::Path, time::SystemTime};

use serde::Serialize;
use textwrap::{core::Word, wrap_algorithms, WordSeparator::UnicodeBreakProperties};
use unicode_width::UnicodeWidthChar;
use fnv::FnvHashMap;
use once_cell::sync::Lazy;

use crate::{core::Gui, il2cpp::{ext::{Il2CppStringExt, StringExt}, hook::umamusume::{Localize, TextId}, types::{Il2CppObject, Il2CppString}, symbols::Thread}};

use super::{Error, Hachimi};

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SendPtr(pub *mut Il2CppObject);

unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

static LOCALIZE_ID_CACHE: Lazy<Mutex<FnvHashMap<String, i32>>> = 
    Lazy::new(|| Mutex::new(FnvHashMap::default()));

pub fn get_localized_string(id_name: &str) -> String {
    let check_cache = |name: &str| -> Option<String> {
        let cache = LOCALIZE_ID_CACHE.lock().unwrap();
        if let Some(&id) = cache.get(name) {
            let ptr = Localize::Get(id);
            if !ptr.is_null() {
                return Some(unsafe { (*ptr).as_utf16str() }.to_string());
            }
            return Some(name.to_owned());
        }
        None
    };

    if let Some(result) = check_cache(id_name) {
        return result;
    }

    let id_name_owned = id_name.to_owned();
    static PENDING_NAME: Mutex<Option<String>> = Mutex::new(None);
    *PENDING_NAME.lock().unwrap() = Some(id_name_owned.clone());

    Thread::main_thread().schedule(|| {
        if let Some(name) = PENDING_NAME.lock().unwrap().take() {
            let val = TextId::from_name(&name);
            LOCALIZE_ID_CACHE.lock().unwrap().insert(name, val);
        }
    });

    check_cache(id_name).unwrap_or_else(|| id_name.to_owned())
}

pub fn char_to_utf16_index(text: &str, char_idx: usize) -> i32 {
    text.chars()
        .take(char_idx)
        .map(|c| c.len_utf16())
        .sum::<usize>() as i32
}

pub fn utf16_to_char_index(text: &str, utf16_idx: usize) -> usize {
    let mut current_utf16_pos = 0;
    let mut char_pos = 0;
    
    for c in text.chars() {
        if current_utf16_pos >= utf16_idx {
            break;
        }
        current_utf16_pos += c.len_utf16();
        char_pos += 1;
    }
    char_pos
}

pub fn str_visual_len(text: &str) -> usize {
    let mut count = 0;
    let mut is_in_tag = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => is_in_tag = true,
            '>' => is_in_tag = false,
            '\\' => {
                if let Some(&'n') = chars.peek() {
                    chars.next();
                } else if !is_in_tag {
                    count += 1;
                }
            }
            _ => {
                if !is_in_tag {
                    count += 1;
                }
            }
        }
    }
    count
}

pub fn concat_unix_path(left: &str, right: &str) -> String {
    let mut str = String::with_capacity(left.len() + 1 + right.len());
    str.push_str(left);
    str.push_str("/");
    str.push_str(right);
    str
}

pub fn print_json_entry(key: &str, value: &str) {
    info!("{}: {},", serde_json::to_string(key).unwrap(), serde_json::to_string(value).unwrap());
}

pub struct IsolateTags<'a> {
    s: &'a str,
    bytes: std::str::Bytes<'a>,
    i: usize,
    current_byte: Option<u8>
}

impl<'a> IsolateTags<'a> {
    pub fn new(s: &'a str) -> Self {
        let mut bytes = s.bytes();
        Self {
            current_byte: bytes.next(),
            s,
            bytes,
            i: 0
        }
    }
}

impl<'a> Iterator for IsolateTags<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_byte.is_none() {
            return None;
        }

        let start = self.i;
        // Unity tags
        let mut tag_start = 0;
        let mut in_tag = false;
        let mut in_closing_tag = false;
        let mut expecting_tag_name = false;
        // Template expressions
        let mut expecting_expr_open = false;
        let mut in_expression = false;

        while let Some(c) = self.current_byte {
            if in_tag {
                match c {
                    b'>' | b'=' | b' ' => 'tag_name_end: {
                        if expecting_tag_name {
                            if !in_closing_tag {
                                // Check for a matching closing tag after
                                let tag_name = &self.s[tag_start+1..self.i];
                                let mut closing_tag = String::with_capacity(3 + tag_name.len());
                                closing_tag += "</";
                                closing_tag += tag_name;
                                closing_tag += ">";
                                if !self.s[self.i..].contains(&closing_tag) {
                                    in_tag = false;
                                    break 'tag_name_end;
                                }
                            }
                            expecting_tag_name = false;
                        }

                        if c == b'>' {
                            // in_tag = false;
                            loop {
                                self.i += 1;
                                self.current_byte = self.bytes.next();
                                if let Some(c) = self.current_byte {
                                    // Capture any whitespace that comes right after it
                                    if char::from(c).is_whitespace() {
                                        continue;
                                    }
                                }
                                break;
                            }
                            return Some((&self.s[start..self.i], false));
                        }
                        else if in_closing_tag {
                            // Invalid character
                            in_tag = false;
                        }
                    }
                    b'/' => {
                        if self.i == tag_start + 1 {
                            in_closing_tag = true;
                        }
                        else if expecting_tag_name {
                            in_tag = false;
                        }
                    }
                    _ => {
                        if expecting_tag_name && !char::from(c).is_ascii_alphabetic() {
                            in_tag = false;
                        }
                    }
                }
            }
            else if in_expression {
                if c == b')'  {
                    if !self.s[self.i..].contains(")") {
                        in_expression = false;
                    }
                    else {
                        loop {
                            self.i += 1;
                            self.current_byte = self.bytes.next();
                            if let Some(c) = self.current_byte {
                                if char::from(c).is_whitespace() {
                                    continue;
                                }
                            }
                            break;
                        }
                        return Some((&self.s[start..self.i], false));
                    }
                }
            }
            else if c == b'<' {
                if start == self.i {
                    in_tag = true;
                    expecting_tag_name = true;
                    tag_start = self.i;
                }
                else {
                    break;
                }
            }
            else if c == b'$' {
                expecting_expr_open = true;
            }
            else if c == b'(' {
                if expecting_expr_open {
                    if self.i != start + 1 {
                        self.i -= 1;
                        self.bytes = self.s.bytes();
                        self.current_byte = self.bytes.nth(self.i);
                        break;
                    }
                    in_expression = true;
                    expecting_expr_open = false;
                }
            }
            else if expecting_expr_open {
                expecting_expr_open = false;
            }

            self.i += 1;
            self.current_byte = self.bytes.next();
        }

        Some((&self.s[start..self.i], true))
    }
}

fn custom_word_separator(line: &str) -> Box<dyn Iterator<Item = Word<'_>> + '_> {
    // Isolate tags and other text (e.g. ['test', '<size=16>', 'hello world', '</size>'])
    // Iter returns str slice and whether to separate words in the section
    // We're only breaking the string on ascii chars, so it's safe to use the bytes
    // iterator and split them based on the index.
    let mut isolate_iter = IsolateTags::new(line);

    let mut unicode_break_iter: Box<dyn Iterator<Item = Word<'_>> + '_> = Box::new(std::iter::empty());
    Box::new(std::iter::from_fn(move || {
        // Continue breaking current split
        let break_res = unicode_break_iter.next();
        if break_res.is_some() {
            return break_res;
        }

        // Advance to next (non-empty) split
        loop {
            if let Some((next_section, needs_break)) = isolate_iter.next() {
                if needs_break {
                    let mut iter = UnicodeBreakProperties.find_words(next_section);
                    let break_res = iter.next();
                    if break_res.is_some() {
                        unicode_break_iter = iter;
                        return break_res;
                    }
                }
                else {
                    unicode_break_iter = Box::new(std::iter::empty());
                    return Some(Word::from(next_section));
                }
            }
            else {
                return None;
            }
        }
    }))
}

fn custom_wrap_algorithm<'a, 'b>(words: &'b [Word<'a>], line_widths: &'b [usize]) -> Vec<&'b [Word<'a>]> {
    // Create intermediate buffer that doesn't contain formatting tags
    let mut clean_fragments = Vec::with_capacity(words.len());
    let mut removed_indices = Vec::with_capacity(words.len());
    let mut remove_offset = 0;
    for (i, word) in words.iter().enumerate() {
        let is_tag = word.starts_with("<") && word.ends_with(">");
        let is_expr = word.starts_with("$(") && word.ends_with(")");
        if is_tag || is_expr {
            removed_indices.push(i - remove_offset);
            remove_offset += 1;
            continue;
        }
        clean_fragments.push(words[i]);
    }

    let config = &Hachimi::instance().localized_data.load();
    let penalties = &config.wrapper_penalties;
    // quick escape!!!11
    let f64_line_widths = line_widths.iter().map(|w| *w as f64).collect::<Vec<_>>();
    if remove_offset == 0 {
        return wrap_algorithms::wrap_optimal_fit(words, &f64_line_widths, penalties).unwrap();
    }

    // Wrap without formatting tags
    let wrapped = wrap_algorithms::wrap_optimal_fit(&clean_fragments, &f64_line_widths, penalties).unwrap();

    // Create results with formatting tags added back
    // Note: The break word option doesn't really affect the extra long lines since
    // the individual tags are separate words (it breaks words, not lines, duh)
    let mut lines = Vec::with_capacity(wrapped.len());
    let mut start = 0;
    let mut clean_start = 0;
    let mut removed_indices_i = 0;
    for (i, line) in wrapped.iter().enumerate() {
        let mut end: usize;
        if i == wrapped.len() - 1 {
            end = words.len();
        }
        else {
            let clean_end = clean_start + line.len();
            end = start + line.len();
            loop {
                let Some(index) = removed_indices.get(removed_indices_i) else {
                    break;
                };
                if *index >= clean_start {
                    if *index < clean_end {
                        end += 1;
                        removed_indices_i += 1;
                    }
                    else {
                        break;
                    }
                }
            }
            clean_start = clean_end;
        }

        lines.push(&words[start..end]);
        start = end;
    }
    lines
}

pub fn wrap_text(string: &str, base_line_width: i32) -> Option<Vec<Cow<'_, str>>> {
    let config = &Hachimi::instance().localized_data.load().config;
    if !config.use_text_wrapper { return None; }
    Some(wrap_text_internal(string, base_line_width, config.line_width_multiplier?))
}

fn wrap_text_internal(string: &str, base_line_width: i32, line_width_multiplier: f32) -> Vec<Cow<'_, str>> {
    let line_width = (base_line_width as f32 * line_width_multiplier).round() as usize;
    let options = textwrap::Options::new(line_width)
        .word_separator(textwrap::WordSeparator::Custom(custom_word_separator))
        .wrap_algorithm(textwrap::WrapAlgorithm::Custom(custom_wrap_algorithm));
    return textwrap::wrap(string, &options);
}

pub fn wrap_text_il2cpp(string: *mut Il2CppString, base_line_width: i32) -> Option<*mut Il2CppString> {
    let config = &Hachimi::instance().localized_data.load().config;
    if !config.use_text_wrapper { return None; }

    Some(
        wrap_text_internal(unsafe { &(*string).as_utf16str().to_string() }, base_line_width, config.line_width_multiplier?)
            .join("\n")
            .to_il2cpp_string()
    )
}

pub fn add_size_tag(string: &str, size: i32) -> String {
    // <size=xx>...</size>
    let mut new_str = String::with_capacity(9 + string.len() + 7);
    new_str.push_str("<size=");
    new_str.push_str(&size.to_string());
    new_str.push_str(">");
    new_str.push_str(string);
    new_str.push_str("</size>");
    new_str
}

pub fn fit_text(string: &str, base_line_width: i32, base_font_size: i32) -> Option<String> {
    let mult = Hachimi::instance().localized_data.load().config.line_width_multiplier?;
    fit_text_internal(string, base_line_width, base_font_size, mult)
}

fn fit_text_internal(
    string: &str, base_line_width: i32, base_font_size: i32, line_width_multiplier: f32
) -> Option<String> {
    let line_width = base_line_width as f32 * line_width_multiplier;

    let count = string.chars().count() as f32;
    if line_width < count {
        Some(add_size_tag(string, (base_font_size as f32 * (line_width / count)) as i32))
    }
    else {
        None
    }
}

pub fn fit_text_il2cpp(string: *mut Il2CppString, base_line_width: i32, base_font_size: i32) -> Option<*mut Il2CppString> {
    let mult = Hachimi::instance().localized_data.load().config.line_width_multiplier?;
    if let Some(result) = fit_text_internal(unsafe { &(*string).as_utf16str().to_string() },
        base_line_width, base_font_size, mult
    ) {
        return Some(result.to_il2cpp_string());
    }

    None
}

// WRAP IT TILL IT FITS GRAHHH BRUTE FORCE GRAHHH
pub fn wrap_fit_text(string: &str, base_line_width: i32, mut max_line_count: i32, base_font_size: i32) -> Option<String> {
    let config = &Hachimi::instance().localized_data.load().config;
    if !config.use_text_wrapper {
        return None;
    }
    let line_width_multiplier = config.line_width_multiplier?;

    // don't wanna mess with different sizes
    if string.contains("<size=") {
        return None;
    }

    let mut line_width = base_line_width as f32;
    let mut font_size = base_font_size as f32;


    loop {
        let wrapped = wrap_text_internal(string, line_width.round() as i32, line_width_multiplier);
        if wrapped.len() as i32 <= max_line_count {
            return Some(add_size_tag(&wrapped.join("\n"), font_size.round() as i32));
        }

        let prev_max_line_count = max_line_count;
        max_line_count += 1;

        let scale = prev_max_line_count as f32 / max_line_count as f32;
        font_size = font_size as f32 * scale;
        line_width = line_width as f32 / scale;
    }
}

pub fn wrap_fit_text_il2cpp(string: *mut Il2CppString, base_line_width: i32, max_line_count: i32, base_font_size: i32) -> Option<*mut Il2CppString> {
    if Hachimi::instance().localized_data.load().config.use_text_wrapper {
        if let Some(result) = wrap_fit_text(unsafe { &(*string).as_utf16str().to_string() },
            base_line_width, max_line_count, base_font_size
        ) {
            return Some(result.to_il2cpp_string());
        }
    }

    None
}

fn truncate_chars_internal(
    mut chars: impl Iterator<Item = char>, mut width: usize, ellipsis: bool, line_width_multiplier: f32
) -> Option<Vec<char>> {
    width = (width as f32 * line_width_multiplier).round() as usize;

    let reserved_width = if ellipsis { width.saturating_sub(1) } else { width };
    let mut v = Vec::with_capacity(width); // it's not the actual max size but it's a good starting point
    let mut total_width = 0;
    let mut dropped_char = None;
    while let Some(c) = chars.next() {
        let char_width = c.width().unwrap_or(0);
        if char_width == 0 {
            v.push(c);
            continue;
        };

        let next_total_width = total_width + char_width;
        if next_total_width > reserved_width {
            dropped_char = Some(c);
            break;
        }

        v.push(c);

        total_width = next_total_width;
        if total_width == reserved_width {
            break;
        }
    }

    if ellipsis {
        // Don't truncate if adding the last dropped or next char would result in the expected width
        let has_next_char = if let Some(c) = dropped_char {
            if total_width + c.width().unwrap_or(0) <= width && chars.next().is_none() {
                return None;
            }
            true
        }
        // doesn't handle control characters correctly but whatever they are never used here
        else if let Some(c) = chars.next() {
            if c.width().unwrap_or(0) <= 1 && chars.next().is_none() {
                return None;
            }
            true
        }
        else {
            false
        };

        // Add ellipsis
        return if has_next_char {
            v.push('…');
            Some(v)
        }
        else {
            None
        }
    }

    if dropped_char.is_some() || chars.next().is_some() {
        Some(v)
    }
    else {
        None
    }
}

pub fn truncate_chars(chars: impl Iterator<Item = char>, width: usize, ellipsis: bool) -> Option<Vec<char>> {
    let line_width_multiplier = Hachimi::instance().localized_data.load().config.line_width_multiplier?;
    truncate_chars_internal(chars, width, ellipsis, line_width_multiplier)
}

pub fn truncate_text_il2cpp(string: *mut Il2CppString, width: usize, ellipsis: bool) -> Option<*mut Il2CppString> {
    let line_width_multiplier = Hachimi::instance().localized_data.load().config.line_width_multiplier?;
    truncate_chars_internal(unsafe { (*string).as_utf16str().chars() }, width, ellipsis, line_width_multiplier).map(|chars|
        chars.iter()
            .collect::<String>()
            .to_il2cpp_string()
    )
}

pub fn write_json_file<T: Serialize, P: AsRef<Path>>(data: &T, path: P) -> Result<(), Error> {
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, data)?;
    writer.flush()?;
    Ok(())
}

// Checks for both \n and \\n
pub fn game_str_has_newline(string: *mut Il2CppString) -> bool {
    let mut got_backslash = false;
    for c in unsafe { (*string).as_utf16str().as_slice().iter() } {
        if got_backslash {
            if *c == 0x6E { // n
                return true;
            }
            got_backslash = false;
        }

        if *c == 0x0A { // newline
            return true;
        }
        else if *c == 0x5C { // backslash
            got_backslash = true; //
        }
    }

    false
}

pub fn scale_to_aspect_ratio(sizes: (i32, i32), aspect_ratio: f32, prefer_larger: bool) -> (i32, i32) {
    let (mut width, mut height) = sizes;
    let orig_aspect_ratio = width as f32 / height as f32;
    // Use original values if possible
    if (aspect_ratio - orig_aspect_ratio).abs() <= 0.001 {
        return sizes;
    }
    else if (aspect_ratio - 1.0/orig_aspect_ratio).abs() <= 0.001 {
        return (height, width);
    }

    let scale_by_height = if prefer_larger { height > width } else { width > height };
    if scale_by_height {
        width = (height as f32 * aspect_ratio).round() as i32;
        // height = height;
    }
    else {
        // width = width;
        height = (width as f32 / aspect_ratio).round() as i32;
    }

    (width, height)
}

pub fn get_file_modified_time<P: AsRef<Path>>(path: P) -> Option<SystemTime> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() { return None; }
    metadata.modified().ok()
}

pub fn get_data_path() -> String {
    #[cfg(target_os = "android")]
    {
        format!("/data/data/{}/files", Hachimi::instance().game.package_name)
    }

    #[cfg(target_os = "windows")]
    {
        unsafe { (*crate::il2cpp::hook::UnityEngine_CoreModule::Application::get_persistentDataPath()).as_utf16str() }.to_string()
    }
}

pub fn get_masterdb_path() -> String {
    format!("{}/master/master.mdb", get_data_path())
}

// Intentionally dumb png loader implementation that only loads RGBA8 images
pub fn load_rgba_png<R: std::io::Read>(r: R) -> Option<(Vec<u8>, png::OutputInfo)> {
    let mut reader = png::Decoder::new(r).read_info().ok()?;
    let mut img_data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut img_data).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    Some((img_data, info))
}

pub fn load_rgba_png_file<P: AsRef<Path>>(path: P) -> Option<(Vec<u8>, png::OutputInfo)> {
    load_rgba_png(File::open(path).ok()?)
}

pub fn notify_error(message: impl AsRef<str>) {
    let s = message.as_ref();
    error!("{}", s);
    if let Some(mutex) = Gui::instance() {
        mutex.lock().unwrap().show_notification(s);
    }
}

pub fn mul_int (base:i32, mult: f32) -> i32 {
    (base as f32 * mult).round() as i32
}
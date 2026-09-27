use std::{collections::HashSet, fs, io::{Cursor, Read, Write}, path::{Path, PathBuf}, sync::Mutex};

use once_cell::sync::Lazy;

use crate::{core::utils::{get_file_modified_time, load_rgba_png_file}, il2cpp::{ext::{Il2CppObjectExt, Il2CppStringExt}, hook::UnityEngine_CoreModule::{Component, RectTransform}, types::*}};

use super::{
    api::{il2cpp_class_get_fields, il2cpp_class_is_enum, il2cpp_field_get_flags, il2cpp_field_get_name},
    hook::{mscorlib, UnityEngine_CoreModule::{Texture, Texture2D},
    UnityEngine_ImageConversionModule::ImageConversion},
    symbols::{get_assembly_image, get_class, get_method_addr_cached, Array}
};

#[allow(dead_code)]
pub fn print_stack_trace() {
    let mscorlib = get_assembly_image(c"mscorlib.dll").expect("mscorlib");
    let environment_class = get_class(mscorlib, c"System", c"Environment").expect("System.Environment");
    let get_fn_addr = get_method_addr_cached(environment_class, c"get_StackTrace", 0);
    let get_fn: extern "C" fn() -> *mut Il2CppString = unsafe { std::mem::transmute(get_fn_addr) };
    debug!("{}", unsafe { (*get_fn()).as_utf16str() });
}

pub fn get_texture_diff_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut diff_path = path.as_ref().to_owned();
    diff_path.set_extension("diff.png");
    diff_path
}

static VERIFIED_SOURCE_CACHES: Lazy<Mutex<HashSet<PathBuf>>> = Lazy::new(|| Mutex::new(HashSet::new()));

fn get_texture_src_hash_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut hash_path = path.as_ref().to_owned();
    let mut name = hash_path.file_name().unwrap_or_default().to_os_string();
    name.push(".srchash");
    hash_path.set_file_name(name);
    hash_path
}

// Hashes Unity Texture2D Color32 pixels using 64-bit FNV-1a (https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function#FNV-1a_hash)
fn hash_color32_pixels(pixels: &[Color32_t]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for pixel in pixels {
        for byte in pixel.as_slice() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn source_texture_hash(texture: *mut Il2CppObject) -> Option<u64> {
    let new_texture = Texture2D::render_to_texture(texture);
    let pixels_array = Texture2D::GetPixels32(new_texture, 0);
    let pixels = unsafe { pixels_array.as_slice() };
    Some(hash_color32_pixels(pixels))
}

fn cached_texture_matches_source(texture: *mut Il2CppObject, path: &Path) -> bool {
    if let Ok(verified) = VERIFIED_SOURCE_CACHES.lock() {
        if verified.contains(path) {
            return true;
        }
    }
    let stored = std::fs::read(get_texture_src_hash_path(path))
        .ok()
        .and_then(|bytes| <[u8; 8]>::try_from(bytes.as_slice()).ok())
        .map(u64::from_le_bytes);
    if let Some(stored) = stored {
        if source_texture_hash(texture) == Some(stored) {
            if let Ok(mut verified) = VERIFIED_SOURCE_CACHES.lock() {
                verified.insert(path.to_path_buf());
            }
            return true;
        }
    }
    false
}

fn store_source_texture_hash(path: &Path, pixels: &[Color32_t]) {
    if let Err(e) = std::fs::write(
        get_texture_src_hash_path(path),
        hash_color32_pixels(pixels).to_le_bytes()
    ) {
        error!("Failed to write texture source hash: {}", e);
    }
}

pub fn replace_texture_with_diff<P: AsRef<Path>>(texture: *mut Il2CppObject, path: P, mark_non_readable: bool) -> bool {
    replace_texture_with_diff_ex(texture, &path, get_texture_diff_path(&path), mark_non_readable, true)
}

pub fn replace_texture_with_diff_ex<P1: AsRef<Path>, P2: AsRef<Path>>(
    texture: *mut Il2CppObject, path: P1, diff_path: P2, mark_non_readable: bool, allow_fallback: bool
) -> bool {
    let Some(diff_mtime) = get_file_modified_time(&diff_path) else {
        // No diff, try to load image directly
        return if allow_fallback {
            Texture2D::load_image_file(texture, &path, mark_non_readable)
        }
        else {
            false
        }
    };

    if let Some(image_mtime) = get_file_modified_time(&path) {
        if diff_mtime < image_mtime && cached_texture_matches_source(texture, path.as_ref()) {
            // Try to load image, otherwise generate it
            // SAFETY: Path has been guaranteed to be a file in mtime check
            if unsafe { Texture2D::load_image_file_unsafe(texture, &path, mark_non_readable) } {
                return true;
            }
        }
    }

    let Some((mut pixels, diff_info)) = load_rgba_png_file(&diff_path) else {
        error!("Failed to load texture diff: {}", diff_path.as_ref().display());
        return false;
    };

    let width = Texture::GetDataWidth(texture) as usize;
    let height = Texture::GetDataHeight(texture) as usize;

    if width as u32 != diff_info.width || height as u32 != diff_info.height {
        error!(
            "Texture diff size mismatch (expected {}x{}, got {}x{}): {}",
            width, height, diff_info.width, diff_info.height, diff_path.as_ref().display()
        );
        return false;
    }

    let new_texture = Texture2D::render_to_texture(texture);
    let orig_pixels_array = Texture2D::GetPixels32(new_texture, 0);
    let orig_pixels = unsafe { orig_pixels_array.as_slice() };

    // Apply diff (reuse/write directly into diff pixels buffer)
    for y in 0..height {
        for x in 0..width {
            let start = (y * width + x) * 4;
            let end = start + 4;
            let pixel = &mut pixels[start..end];
            if pixel[3] == 0 {
                // Use original pixel if diff pixel is transparent
                // Original image is flipped
                let orig_pixel = &orig_pixels[(height - y - 1) * width + x];
                pixel.copy_from_slice(orig_pixel.as_slice());
            }
            else if pixel == [255, 0, 255, 255] {
                // Make pixel transparent if it's #FF00FF
                pixel.fill(0);
            }
            // else keep the diff pixel
        }
    }

    // 1MiB should be enough for most images
    let mut png_buffer = Vec::with_capacity(std::cmp::min(pixels.len(), 1048576));
    let mut encoder = png::Encoder::new(&mut png_buffer, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);

    { // Scope to drop writer and release borrow to png buffer
        let mut writer = match encoder.write_header() {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to write PNG header: {}", e);
                return false;
            }
        };

        if let Err(e) = writer.write_image_data(&pixels) {
            error!("Failed to write PNG image: {}", e);
            return false;
        }
    }

    // Reclaim some memory...
    std::mem::drop(pixels);

    // Create output dir
    let Some(path_dir) = path.as_ref().parent() else {
        return false;
    };
    match std::fs::create_dir_all(path_dir) {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create directory: {}", e);
            return false;
        }
    }

    // Write to file
    let mut out_file = match std::fs::File::create(&path) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to create file: {}", e);
            return false;
        }
    };

    if let Err(e) = out_file.write(&png_buffer) {
        error!("Failed to write to file: {}", e);
        return false;
    }

    store_source_texture_hash(path.as_ref(), orig_pixels);

    // And finally load image to texture
    let png_array = Array::<u8>::new(mscorlib::Byte::class(), png_buffer.len());
    unsafe { png_array.as_slice().copy_from_slice(&png_buffer); }
    ImageConversion::LoadImage(texture, png_array.this, mark_non_readable);

    true
}

/// Changes the "active area" of the GameObject a Component is part of.
/// Numbers <=0 skip that axis.
pub fn adjust_transform_size(component: *mut Il2CppObject, width: f32, height: f32) {
    let transform = Component::get_transform(component);
    if unsafe { (*transform).klass() } == RectTransform::class() {
        if width > 0.0 {
            RectTransform::SetSizeWithCurrentAnchors(transform, RectTransform::Axis::Horizontal, width);
        }
        if height > 0.0 {
            RectTransform::SetSizeWithCurrentAnchors(transform, RectTransform::Axis::Vertical, height);
        }
    }
}

pub fn umamusume_enum_options(class_name: &std::ffi::CStr) -> Vec<String> {
    let mut options = Vec::new();
    let Ok(image) = get_assembly_image(c"umamusume.dll") else { return options };
    let Ok(klass) = get_class(image, c"Gallop", class_name) else { return options };

    if !il2cpp_class_is_enum(klass) { return options; }

    let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
    loop {
        let field = il2cpp_class_get_fields(klass, &mut iter);
        if field.is_null() { break; }
        let attrs = il2cpp_field_get_flags(field);
        if (attrs & 0x0040) != 0 {
            let name_ptr = il2cpp_field_get_name(field);
            if !name_ptr.is_null() {
                let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
                if let Ok(s) = name.to_str() {
                    options.push(s.to_string());
                }
            }
        }
    }
    options
}

const FONT_PATH_ENTRY: &str = "font_path.txt";
#[cfg(target_os = "android")]
const INCLUDE_ENTRY: &str = "includes_android";
#[cfg(target_os = "windows")]
const INCLUDE_ENTRY: &str = "includes_win";
const MAX_FONT_PATH_BYTES: u64 = 4096;
const MAX_BUNDLE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const BUNDLE_MAGICS: [&[u8]; 4] = [b"UnityFS", b"UnityRaw", b"UnityWeb", b"UnityArchive"];

pub fn read_font_pack(path: &Path) -> Result<(Vec<u8>, String), String> {
    let data = fs::read(path).map_err(|e| format!("failed to open: {e}"))?;
    read_font_pack_data(&data)
}

fn read_font_pack_data(data: &[u8]) -> Result<(Vec<u8>, String), String> {
    if data.is_empty() {
        return Err("file is empty".to_owned());
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(data))
        .map_err(|e| format!("invalid zip file: {e}"))?;

    let font_path_entry = archive.file_names()
        .find(|name| name.eq_ignore_ascii_case(FONT_PATH_ENTRY))
        .ok_or_else(|| format!("missing {FONT_PATH_ENTRY}"))?
        .to_owned();
    let bundle_entry = archive.file_names()
        .find(|name| name.eq_ignore_ascii_case(INCLUDE_ENTRY))
        .ok_or_else(|| format!("missing {INCLUDE_ENTRY}"))?
        .to_owned();

    let mut content = Vec::new();
    {
        let mut font_path = archive.by_name(&font_path_entry)
            .map_err(|e| format!("failed to read {FONT_PATH_ENTRY}: {e}"))?;
        if font_path.size() > MAX_FONT_PATH_BYTES {
            return Err(format!("{FONT_PATH_ENTRY} is too large"));
        }
        font_path.by_ref().take(MAX_FONT_PATH_BYTES + 1).read_to_end(&mut content)
            .map_err(|e| format!("failed to read {FONT_PATH_ENTRY}: {e}"))?;
    }
    if content.len() as u64 > MAX_FONT_PATH_BYTES {
        return Err(format!("{FONT_PATH_ENTRY} is too large"));
    }
    let text = String::from_utf8(content)
        .map_err(|_| format!("{FONT_PATH_ENTRY} is not valid UTF-8"))?;
    let text = text.trim();
    let text = text.strip_prefix('\u{feff}').map_or(text, str::trim);
    if text.is_empty() {
        return Err(format!("{FONT_PATH_ENTRY} is empty"));
    }
    if text.chars().any(char::is_control) {
        return Err(format!("{FONT_PATH_ENTRY} contains control characters"));
    }
    let asset_path = text.replace('\\', "/").to_lowercase();

    let mut bundle = archive.by_name(&bundle_entry)
        .map_err(|e| format!("failed to read asset bundle entry {bundle_entry}: {e}"))?;
    if bundle.size() == 0 {
        return Err(format!("asset bundle entry {bundle_entry} is empty"));
    }
    if bundle.size() > MAX_BUNDLE_BYTES {
        return Err(format!("asset bundle entry {bundle_entry} is too large"));
    }
    let mut bytes = Vec::new();
    bundle.by_ref().take(MAX_BUNDLE_BYTES + 1).read_to_end(&mut bytes)
        .map_err(|e| format!("failed to read asset bundle entry {bundle_entry}: {e}"))?;
    if bytes.len() as u64 > MAX_BUNDLE_BYTES {
        return Err(format!("asset bundle entry {bundle_entry} is too large"));
    }
    if !BUNDLE_MAGICS.iter().any(|m| bytes.starts_with(m)) {
        return Err(format!("asset bundle entry {bundle_entry} is not a Unity AssetBundle"));
    }

    Ok((bytes, asset_path))
}

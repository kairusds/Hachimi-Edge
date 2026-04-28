use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::os::windows::ffi::OsStrExt;
use once_cell::sync::Lazy;
use windows::{
    core::{factory, Interface, HSTRING, PCWSTR},
    Media::{
        SystemMediaTransportControls, SystemMediaTransportControlsButton,
        SystemMediaTransportControlsButtonPressedEventArgs, MediaPlaybackType, MediaPlaybackStatus
    },
    Storage::Streams::{InMemoryRandomAccessStream, DataWriter, RandomAccessStreamReference},
    Win32::{
        Foundation::HWND,
        System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, IPersistFile},
        System::WinRT::ISystemMediaTransportControlsInterop,
        UI::Shell::{SHGetKnownFolderPath, FOLDERID_Programs, KF_FLAG_DEFAULT}
    }
};

use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppStringExt, StringExt, Il2CppObjectExt},
        hook::{
            UnityEngine_CoreModule::{Texture2D, RenderTexture, Graphics, Texture, SceneManager, Scene},
            UnityEngine_ImageConversionModule::ImageConversion
        },
        types::*
    }
};

static SMTC_INSTANCE: Lazy<Mutex<Option<SystemMediaTransportControls>>> = Lazy::new(|| Mutex::new(None));
static mut CURRENT_SCENE_HANDLE: i32 = -1;
static mut CURRENT_MUSIC_ID: i32 = -1;
static PENDING_BUTTON: AtomicU32 = AtomicU32::new(u32::MAX);
static mut IS_LIVE_SCENE: bool = false;
static mut IS_HOME_SCENE: bool = false;

static mut TASKBAR_HWND: HWND = HWND(std::ptr::null_mut());

static mut LAST_STATUS: MediaPlaybackStatus = MediaPlaybackStatus::Closed;
static mut LAST_HOME_MUSIC_ID: i32 = 0;

fn set_playback_status(smtc: &SystemMediaTransportControls, status: MediaPlaybackStatus) {
    unsafe {
        if LAST_STATUS != status {
            let _ = smtc.SetPlaybackStatus(status);
            LAST_STATUS = status;
        }
    }
}

pub fn init(hwnd: HWND) {
    if !Hachimi::instance().config.load().windows.enable_smtc {
        return;
    }
    unsafe {
        TASKBAR_HWND = hwnd;
    }
}

unsafe fn create_shortcut(name: &str) {
    if let Ok(shell_link) = CoCreateInstance::<_, windows::Win32::UI::Shell::IShellLinkW>(&windows::Win32::UI::Shell::ShellLink, None, CLSCTX_INPROC_SERVER) {
        let exe_path = crate::windows::utils::get_exec_path();
        let mut exe_path_u16: Vec<u16> = exe_path.as_os_str().encode_wide().collect();
        exe_path_u16.push(0);
        shell_link.SetPath(PCWSTR::from_raw(exe_path_u16.as_ptr())).unwrap();

        let work_dir = exe_path.parent().unwrap();
        let mut work_dir_u16: Vec<u16> = work_dir.as_os_str().encode_wide().collect();
        work_dir_u16.push(0);
        shell_link.SetWorkingDirectory(PCWSTR::from_raw(work_dir_u16.as_ptr())).unwrap();

        if let Ok(property_store) = shell_link.cast::<windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore>() {
            let pkey = windows::Win32::Foundation::PROPERTYKEY {
                fmtid: windows::core::GUID::from_values(0x9F4C2855, 0x9F79, 0x4B39,[0xA8, 0xD0, 0xE1, 0xD4, 0x2D, 0xE1, 0xD5, 0xF3]),
                pid: 5,
            };

            let mut aumid_u16: Vec<u16> = "Cygames.Gallop".encode_utf16().collect();
            aumid_u16.push(0);

            #[repr(C)]
            struct PropVariantLayout {
                vt: u16,
                wReserved1: u16,
                wReserved2: u16,
                wReserved3: u16,
                data: *mut u16,
            }

            let mut pv = std::mem::ManuallyDrop::new(windows::Win32::System::Com::StructuredStorage::PROPVARIANT::default());
            unsafe {
                let size_bytes = aumid_u16.len() * 2;
                let alloc_ptr = windows::Win32::System::Com::CoTaskMemAlloc(size_bytes);
                if !alloc_ptr.is_null() {
                    std::ptr::copy_nonoverlapping(aumid_u16.as_ptr(), alloc_ptr as *mut u16, aumid_u16.len());

                    let pv_layout = &mut *pv as *mut _ as *mut PropVariantLayout;
                    (*pv_layout).vt = 31;
                    (*pv_layout).data = alloc_ptr as *mut u16;

                    let _ = property_store.SetValue(&pkey, &*pv);
                    let _ = property_store.Commit();

                    let _ = windows::Win32::System::Com::StructuredStorage::PropVariantClear(&mut *pv);
                }
            }
        }

        if let Ok(persist_file) = shell_link.cast::<IPersistFile>() {
            if let Ok(pwstr) = SHGetKnownFolderPath(&FOLDERID_Programs, KF_FLAG_DEFAULT, None) {
                let mut lnk_path = pwstr.to_string().unwrap();
                windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.as_ptr() as _));
                lnk_path.push_str(&format!("\\{}.lnk", name));
                if std::path::Path::new(&lnk_path).exists() {
                    let _ = std::fs::remove_file(&lnk_path);
                }
                let mut wide_path: Vec<u16> = lnk_path.encode_utf16().collect();
                wide_path.push(0);
                persist_file.Save(PCWSTR::from_raw(wide_path.as_ptr()), true).unwrap();
            }
        }
    }
}

pub fn on_update() {
    if !crate::core::Hachimi::instance().config.load().windows.enable_smtc {
        return;
    }
    let mut smtc_guard = SMTC_INSTANCE.lock().unwrap();
    if smtc_guard.is_none() {
        unsafe {
            let hwnd = TASKBAR_HWND;
            if hwnd.0.is_null() { return; }

            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
            let title = if crate::core::Hachimi::instance().game.region == crate::core::game::Region::Japan {
                "ウマ娘"
            } else {
                "Umamusume"
            };
            create_shortcut(&format!("{} (Hachimi)", title));
            if let Ok(interop) = factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>() {
                if let Ok(smtc) = interop.GetForWindow::<SystemMediaTransportControls>(hwnd) {
                    let _ = smtc.SetIsEnabled(false);
                    if let Ok(updater) = smtc.DisplayUpdater() {
                        let _ = updater.SetType(MediaPlaybackType::Music);
                        let _ = updater.Update();
                    }
                    let handler = windows::Foundation::TypedEventHandler::<SystemMediaTransportControls, SystemMediaTransportControlsButtonPressedEventArgs>::new(
                        |_sender, args| {
                            if let Some(args_ref) = args.as_ref() {
                                handle_button_pressed(args_ref);
                            }
                            Ok(())
                        }
                    );
                    let _ = smtc.ButtonPressed(&handler);
                    *smtc_guard = Some(smtc);
                }
            }
        }
    }
    let smtc = if let Some(s) = smtc_guard.as_ref() { s } else { return; };

    let scene = SceneManager::GetActiveScene();
    let current_scene_handle = unsafe { CURRENT_SCENE_HANDLE };

    if scene.handle != current_scene_handle {
        unsafe { CURRENT_SCENE_HANDLE = scene.handle; }

        let name_ptr = Scene::GetNameInternal(scene.handle);
        let name = if name_ptr.is_null() { String::new() } else { unsafe { (*name_ptr).as_utf16str().to_string() } };

        unsafe {
            IS_LIVE_SCENE = name == "Live";
            IS_HOME_SCENE = name == "Home";
        }

        if name == "Live" {
            let _ = smtc.SetIsEnabled(true);
            if let Some(music_id) = get_live_music_id() {
                update_metadata(smtc, music_id);
            }
            let _ = smtc.SetIsPreviousEnabled(false);
            let _ = smtc.SetIsNextEnabled(true);
        } else if name == "Home" {
            let _ = smtc.SetIsEnabled(true);
            let _ = smtc.SetIsPreviousEnabled(false);
            let _ = smtc.SetIsNextEnabled(false);
            update_metadata_home(smtc);
        } else {
            let _ = smtc.SetIsEnabled(false);
            let _ = smtc.SetIsPreviousEnabled(false);
            let _ = smtc.SetIsNextEnabled(false);
        }
    }

    if unsafe { IS_LIVE_SCENE } {
        update_live_playback(smtc);
    } else if unsafe { IS_HOME_SCENE } {
        update_metadata_home(smtc);
    }
}

fn get_live_music_id() -> Option<i32> {
    let image = crate::il2cpp::symbols::get_assembly_image(c"umamusume.dll").ok()?;
    let klass = crate::il2cpp::symbols::get_class(image, c"Gallop.Live", c"Director").ok()?;

    let get_load_settings_addr = crate::il2cpp::symbols::get_method_addr_cached(klass, c"get_LoadSettings", 0);
    if get_load_settings_addr == 0 { return None; }

    let get_load_settings: extern "C" fn() -> *mut crate::il2cpp::types::Il2CppObject = unsafe { std::mem::transmute(get_load_settings_addr) };
    let load_settings = get_load_settings();
    if load_settings.is_null() { return None; }

    let get_music_id_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*load_settings).klass() }, c"get_MusicId", 0);
    if get_music_id_addr == 0 { return None; }

    let get_music_id: extern "C" fn(*mut Il2CppObject) -> i32 = unsafe { std::mem::transmute(get_music_id_addr) };
    Some(get_music_id(load_settings))
}

fn get_jacket_texture(music_id: i32) -> Option<*mut Il2CppObject> {
    let image = crate::il2cpp::symbols::get_assembly_image(c"umamusume.dll").ok()?;
    let am_klass = crate::il2cpp::symbols::get_class(image, c"Gallop", c"AssetManager").ok()?;

    let get_loader_addr = crate::il2cpp::symbols::get_method_addr_cached(am_klass, c"get_Loader", 0);
    if get_loader_addr == 0 { return None; }

    let get_loader: extern "C" fn() -> *mut Il2CppObject = unsafe { std::mem::transmute(get_loader_addr) };
    let loader = get_loader();
    if loader.is_null() { return None; }

    let load_asset_handle_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*loader).klass() }, c"LoadAssetHandle", 2);
    if load_asset_handle_addr == 0 { return None; }

    let load_asset_handle: extern "C" fn(*mut Il2CppObject, *mut Il2CppString, bool) -> *mut Il2CppObject = unsafe { std::mem::transmute(load_asset_handle_addr) };

    let music_id_str = format!("{:04}", music_id);
    let jacket_name = format!("jacket_icon_m_{}", music_id_str);
    let path = format!("Live/Jacket/{}", jacket_name);

    let asset_handle = load_asset_handle(loader, path.to_il2cpp_string(), false);
    if asset_handle.is_null() { return None; }

    let get_asset_bundle_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*asset_handle).klass() }, c"get_assetBundle", 0);
    if get_asset_bundle_addr == 0 { return None; }

    let get_asset_bundle: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_asset_bundle_addr) };
    let bundle = get_asset_bundle(asset_handle);
    if bundle.is_null() { return None; }

    let t2d_klass = crate::il2cpp::hook::UnityEngine_CoreModule::Texture2D::class();
    let t2d_type = crate::il2cpp::api::il2cpp_class_get_type(t2d_klass);
    let t2d_type_obj = crate::il2cpp::api::il2cpp_type_get_object(t2d_type);

    let load_asset_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*bundle).klass() }, c"LoadAsset", 2);
    if load_asset_addr == 0 { return None; }

    let load_asset: extern "C" fn(*mut Il2CppObject, *mut Il2CppString, *mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(load_asset_addr) };

    let tex = load_asset(bundle, jacket_name.to_il2cpp_string(), t2d_type_obj);
    if tex.is_null() { return None; }

    Some(tex)
}

fn generate_thumbnail_and_update(texture: *mut Il2CppObject, smtc: &SystemMediaTransportControls) -> Option<()> {
    unsafe {
        let width = Texture::GetDataWidth(texture);
        let height = Texture::GetDataHeight(texture);

        let render_texture = RenderTexture::GetTemporary(width, height);
        Graphics::Blit2(texture, render_texture);
        let prev_active = RenderTexture::GetActive();
        RenderTexture::SetActive(render_texture);

        let readable_texture = Texture2D::new(width, height);
        Texture2D::ReadPixels(readable_texture, Rect_t { x: 0.0, y: 0.0, width: width as f32, height: height as f32 }, 0, 0);
        Texture2D::Apply(readable_texture);

        RenderTexture::SetActive(prev_active);
        RenderTexture::ReleaseTemporary(render_texture);

        let png_array_size = ImageConversion::EncodeToPNG(readable_texture);
        if png_array_size.is_null() { return None; }

        let png_data = std::slice::from_raw_parts(
            ((*png_array_size).vector).as_ptr() as *const u8,
            (*png_array_size).max_length as usize
        ).to_vec();

        let smtc_clone = smtc.clone();
        std::thread::spawn(move || {
            let _ = (|| -> Option<()> {
                let stream = InMemoryRandomAccessStream::new().ok()?;
                let writer = DataWriter::CreateDataWriter(&stream).ok()?;
                writer.WriteBytes(&png_data).ok()?;

                let _ = writer.StoreAsync().ok()?.join().ok()?;

                if let Ok(updater) = smtc_clone.DisplayUpdater() {
                    if let Ok(stream_ref) = RandomAccessStreamReference::CreateFromStream(&stream) {
                        let _ = updater.SetThumbnail(&stream_ref).ok();
                        let _ = updater.Update().ok();
                    }
                }
                Some(())
            })();
        });

        Some(())
    }
}

fn update_metadata(smtc: &SystemMediaTransportControls, music_id: i32) {
    if music_id == 0 { return; }

    unsafe {
        if CURRENT_MUSIC_ID != music_id {
            if let Ok(updater) = smtc.DisplayUpdater() {
                let _ = updater.SetThumbnail(None);
                let _ = updater.Update();
            }
        }
        CURRENT_MUSIC_ID = music_id;
    }

    let _ = smtc.SetIsPlayEnabled(true);
    let _ = smtc.SetIsPauseEnabled(true);

    if let Ok(updater) = smtc.DisplayUpdater() {
        if let Ok(props) = updater.MusicProperties() {
            if let Some(title) = crate::il2cpp::sql::get_master_text(16, music_id) {
                if !title.is_empty() {
                    let _ = props.SetTitle(&HSTRING::from(title));
                }
            }
            if let Some(artist) = crate::il2cpp::sql::get_master_text(17, music_id) {
                if !artist.is_empty() {
                    let artist_str = artist.replace("\\n", ", ");
                    let _ = props.SetArtist(&HSTRING::from(artist_str));
                }
            }
        }

        if let Ok(_thumbnail) = updater.Thumbnail() {
        } else {
            if let Some(tex) = get_jacket_texture(music_id) {
                generate_thumbnail_and_update(tex, smtc);
            }
        }
        let _ = updater.Update();
    }
}

fn update_metadata_home(smtc: &SystemMediaTransportControls) {
    let mut has_set_list = false;
    let hub_vc = get_current_hub_view_child_controller();
    if !hub_vc.is_null() {
        let hub_name = unsafe { std::ffi::CStr::from_ptr((*(*hub_vc).klass()).name) }.to_string_lossy();
        if hub_name == "HomeViewController" {
            let get_top_ui_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*hub_vc).klass() }, c"GetTopUI", 1);
            if get_top_ui_addr != 0 {
                let get_top_ui: extern "C" fn(*mut crate::il2cpp::types::Il2CppObject, i32) -> *mut crate::il2cpp::types::Il2CppObject = unsafe { std::mem::transmute(get_top_ui_addr) };
                let top_ui = get_top_ui(hub_vc, 10);
                if !top_ui.is_null() {
                    let get_data_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*top_ui).klass() }, c"get_TempSetListPlayingData", 0);
                    if get_data_addr != 0 {
                        let get_data: extern "C" fn(*mut crate::il2cpp::types::Il2CppObject) -> *mut crate::il2cpp::types::Il2CppObject = unsafe { std::mem::transmute(get_data_addr) };
                        let data = get_data(top_ui);
                        if !data.is_null() {
                            let is_playing_field = crate::il2cpp::symbols::get_field_from_name(unsafe { (*data).klass() }, c"IsPlaying");
                            if !is_playing_field.is_null() {
                                let is_playing = crate::il2cpp::symbols::get_field_value::<bool>(data, is_playing_field);
                                if is_playing {
                                    has_set_list = true;
                                    set_playback_status(smtc, MediaPlaybackStatus::Playing);
                                    let set_list_index_field = crate::il2cpp::symbols::get_field_from_name(unsafe { (*data).klass() }, c"SetListIndex");
                                    let set_list_index = if !set_list_index_field.is_null() {
                                        crate::il2cpp::symbols::get_field_value::<i32>(data, set_list_index_field)
                                    } else { 0 };

                                    let get_music_list_count_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*data).klass() }, c"GetMusicListCount", 0);
                                    let music_list_count = if get_music_list_count_addr != 0 {
                                        let get_music_list_count: extern "C" fn(*mut crate::il2cpp::types::Il2CppObject) -> i32 = unsafe { std::mem::transmute(get_music_list_count_addr) };
                                        get_music_list_count(data)
                                    } else { 0 };

                                    let _ = smtc.SetIsPreviousEnabled(set_list_index > 0);
                                    let _ = smtc.SetIsNextEnabled(set_list_index < music_list_count - 1);

                                    let get_master_set_list_music_data_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*data).klass() }, c"GetMasterSetListMusicData", 0);
                                    if get_master_set_list_music_data_addr != 0 {
                                        let get_master_set_list_music_data: extern "C" fn(*mut crate::il2cpp::types::Il2CppObject) -> *mut crate::il2cpp::types::Il2CppObject = unsafe { std::mem::transmute(get_master_set_list_music_data_addr) };
                                        let music_data = get_master_set_list_music_data(data);
                                        if !music_data.is_null() {
                                            let music_id_field = crate::il2cpp::symbols::get_field_from_name(unsafe { (*music_data).klass() }, c"MusicId");
                                            if !music_id_field.is_null() {
                                                let music_id = crate::il2cpp::symbols::get_field_value::<i32>(music_data, music_id_field);
                                                unsafe { LAST_HOME_MUSIC_ID = music_id; }
                                                update_metadata(smtc, music_id);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !has_set_list {
        let image = match crate::il2cpp::symbols::get_assembly_image(c"umamusume.dll") {
            Ok(img) => img,
            Err(_) => return
        };
        let klass = match crate::il2cpp::symbols::get_class(image, c"Gallop", c"WorkDataManager") {
            Ok(k) => k,
            Err(_) => return
        };

        let instance = match crate::il2cpp::symbols::SingletonLike::new(klass) {
            Some(s) => s.instance(),
            None => return
        };
        if instance.is_null() { return; }

        let get_jukebox_addr = crate::il2cpp::symbols::get_method_addr_cached(klass, c"get_JukeboxData", 0);
        if get_jukebox_addr == 0 { return; }

        let get_jukebox: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_jukebox_addr) };
        let jukebox = get_jukebox(instance);
        if jukebox.is_null() { return; }

        let get_music_id_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*jukebox).klass() }, c"GetCurrentBgmMusicId", 0);
        if get_music_id_addr == 0 { return; }

        let get_music_id: extern "C" fn(*mut Il2CppObject) -> i32 = unsafe { std::mem::transmute(get_music_id_addr) };
        let music_id = get_music_id(jukebox);

        if music_id != 0 {
            unsafe { LAST_HOME_MUSIC_ID = music_id; }
            update_metadata(smtc, music_id);
            set_playback_status(smtc, MediaPlaybackStatus::Playing);
        } else {
            unsafe {
                if LAST_HOME_MUSIC_ID != 0 {
                    update_metadata(smtc, LAST_HOME_MUSIC_ID);
                }
            }
            set_playback_status(smtc, MediaPlaybackStatus::Paused);
        }
    }
}

fn update_live_playback(smtc: &SystemMediaTransportControls) {
    let vc = get_current_view_controller();
    if !vc.is_null() {
        let name = unsafe { std::ffi::CStr::from_ptr((*(*vc).klass()).name) }.to_string_lossy();
        if name == "LiveViewController" {
            let state_field = crate::il2cpp::symbols::get_field_from_name(unsafe { (*vc).klass() }, c"_state");
            if !state_field.is_null() {
                let state = crate::il2cpp::symbols::get_field_value::<i32>(vc, state_field);
                let status = if state == 0 { MediaPlaybackStatus::Playing } else { MediaPlaybackStatus::Paused };
                set_playback_status(smtc, status);
                return;
            }
        }
    }

    let image = match crate::il2cpp::symbols::get_assembly_image(c"umamusume.dll") {
        Ok(img) => img,
        Err(_) => return
    };
    let klass = match crate::il2cpp::symbols::get_class(image, c"Gallop.Live", c"Director") {
        Ok(k) => k,
        Err(_) => return
    };

    let instance = match crate::il2cpp::symbols::SingletonLike::new(klass) {
        Some(s) => s.instance(),
        None => return
    };
    if instance.is_null() { return; }

    let is_pause_addr = crate::il2cpp::symbols::get_method_addr_cached(klass, c"IsPauseLive", 0);
    if is_pause_addr == 0 { return; }

    let is_pause: extern "C" fn(*mut Il2CppObject) -> bool = unsafe { std::mem::transmute(is_pause_addr) };
    let paused = is_pause(instance);
    let status = if paused { MediaPlaybackStatus::Paused } else { MediaPlaybackStatus::Playing };

    set_playback_status(smtc, status);
}

fn get_current_view_controller() -> *mut Il2CppObject {
    let image = match crate::il2cpp::symbols::get_assembly_image(c"umamusume.dll") {
        Ok(img) => img,
        Err(_) => return std::ptr::null_mut()
    };
    let klass = match crate::il2cpp::symbols::get_class(image, c"Gallop", c"SceneManager") {
        Ok(k) => k,
        Err(_) => return std::ptr::null_mut()
    };
    let instance = match crate::il2cpp::symbols::SingletonLike::new(klass) {
        Some(s) => s.instance(),
        None => return std::ptr::null_mut()
    };

    let mut get_vc_addr = 0;
    let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
    loop {
        let method = crate::il2cpp::api::il2cpp_class_get_methods(klass, &mut iter);
        if method.is_null() { break; }
        let name = unsafe { std::ffi::CStr::from_ptr((*method).name) }.to_string_lossy();
        if name == "GetCurrentViewController" {
            if unsafe { (*method).is_generic() } == 0 {
                get_vc_addr = unsafe { (*method).methodPointer };
                break;
            }
        }
    }
    if get_vc_addr == 0 { return std::ptr::null_mut(); }

    let get_vc: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_vc_addr) };
    get_vc(instance)
}

fn get_current_hub_view_child_controller() -> *mut Il2CppObject {
    let vc = get_current_view_controller();
    if vc.is_null() { return std::ptr::null_mut(); }
    let klass = unsafe { (*vc).klass() };
    let parent_klass = unsafe { (*klass).parent };
    if parent_klass.is_null() { return std::ptr::null_mut(); }
    let parent_name = unsafe { std::ffi::CStr::from_ptr((*parent_klass).name) }.to_string_lossy();
    if parent_name == "HubViewControllerBase" {
        let get_child_addr = crate::il2cpp::symbols::get_method_addr_cached(klass, c"get_ChildCurrentController", 0);
        if get_child_addr != 0 {
            let get_child: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_child_addr) };
            return get_child(vc);
        }
    }
    std::ptr::null_mut()
}

fn handle_button_pressed(args: &SystemMediaTransportControlsButtonPressedEventArgs) {
    if let Ok(button) = args.Button() {
        PENDING_BUTTON.store(button.0 as u32, Ordering::Relaxed);
        crate::il2cpp::symbols::Thread::main_thread().schedule(move || {
            let button_val = PENDING_BUTTON.swap(u32::MAX, Ordering::Relaxed);
            if button_val == u32::MAX { return; }
            let button = SystemMediaTransportControlsButton(button_val as i32);

            let vc = get_current_view_controller();
            if !vc.is_null() {
                let name = unsafe { std::ffi::CStr::from_ptr((*(*vc).klass()).name) }.to_string_lossy();
                if name == "LiveViewController" {
                    if button == SystemMediaTransportControlsButton::Pause {
                        let method = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*vc).klass() }, c"PauseLive", 0);
                        if method != 0 {
                            let pause: extern "C" fn(*mut Il2CppObject) = unsafe { std::mem::transmute(method) };
                            pause(vc);
                        }
                    } else if button == SystemMediaTransportControlsButton::Play {
                        let method = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*vc).klass() }, c"ResumeLive", 0);
                        if method != 0 {
                            let resume: extern "C" fn(*mut Il2CppObject) = unsafe { std::mem::transmute(method) };
                            resume(vc);
                        }
                    } else if button == SystemMediaTransportControlsButton::Next {
                        let method = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*vc).klass() }, c"SkipLive", 0);
                        if method != 0 {
                            let skip: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(method) };
                            let coroutine = skip(vc);

                            let start_coroutine_addr = crate::il2cpp::api::il2cpp_resolve_icall(c"UnityEngine.MonoBehaviour::StartCoroutineManaged2()".as_ptr());
                            if start_coroutine_addr != 0 {
                                let start_coroutine: extern "C" fn(*mut Il2CppObject, *mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(start_coroutine_addr) };

                                let get_view_base_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*vc).klass() }, c"GetViewBase", 0);
                                if get_view_base_addr != 0 {
                                    let get_view_base: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_view_base_addr) };
                                    let view = get_view_base(vc);
                                    if !view.is_null() {
                                        start_coroutine(view, coroutine);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let hub_vc = get_current_hub_view_child_controller();
            if !hub_vc.is_null() {
                let hub_name = unsafe { std::ffi::CStr::from_ptr((*(*hub_vc).klass()).name) }.to_string_lossy();
                if hub_name == "HomeViewController" {
                    let get_top_ui_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*hub_vc).klass() }, c"GetTopUI", 1);
                    if get_top_ui_addr != 0 {
                        let get_top_ui: extern "C" fn(*mut Il2CppObject, i32) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_top_ui_addr) };
                        let top_ui = get_top_ui(hub_vc, 10);
                        if !top_ui.is_null() {
                            if button == SystemMediaTransportControlsButton::Play {
                                let get_data_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*top_ui).klass() }, c"get_TempSetListPlayingData", 0);
                                if get_data_addr != 0 {
                                    let get_data: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_data_addr) };
                                    let data = get_data(top_ui);
                                    let mut is_set_list = false;
                                    if !data.is_null() {
                                        let set_list_id_field = crate::il2cpp::symbols::get_field_from_name(unsafe { (*data).klass() }, c"SetListId");
                                        if !set_list_id_field.is_null() {
                                            let set_list_id = crate::il2cpp::symbols::get_field_value::<i32>(data, set_list_id_field);
                                            if set_list_id > 0 {
                                                is_set_list = true;
                                                let selector_field = crate::il2cpp::symbols::get_field_from_name(unsafe { (*top_ui).klass() }, c"_jukeboxBgmSelector");
                                                if !selector_field.is_null() {
                                                    let selector = crate::il2cpp::symbols::get_field_object_value::<Il2CppObject>(top_ui, selector_field);
                                                    if !selector.is_null() {
                                                        let play_set_list_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*selector).klass() }, c"PlayCoroutinePlaySetList", 3);
                                                        if play_set_list_addr != 0 {
                                                            let play_set_list: extern "C" fn(*mut Il2CppObject, bool, f32, bool) = unsafe { std::mem::transmute(play_set_list_addr) };
                                                            play_set_list(selector, true, 0.0, true);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if !is_set_list {
                                        let set_play_flag_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*top_ui).klass() }, c"SetPlayMusicFlag", 1);
                                        if set_play_flag_addr != 0 {
                                            let set_play_flag: extern "C" fn(*mut Il2CppObject, bool) = unsafe { std::mem::transmute(set_play_flag_addr) };
                                            set_play_flag(top_ui, true);
                                        } else {
                                            let play_req_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*top_ui).klass() }, c"PlayRequestSong", 0);
                                            if play_req_addr != 0 {
                                                let play_req: extern "C" fn(*mut Il2CppObject) = unsafe { std::mem::transmute(play_req_addr) };
                                                play_req(top_ui);
                                            }
                                        }
                                    }
                                }
                            } else if button == SystemMediaTransportControlsButton::Pause {
                                let set_play_flag_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*top_ui).klass() }, c"SetPlayMusicFlag", 1);
                                if set_play_flag_addr != 0 {
                                    let set_play_flag: extern "C" fn(*mut Il2CppObject, bool) = unsafe { std::mem::transmute(set_play_flag_addr) };
                                    set_play_flag(top_ui, false);
                                }
                            } else if button == SystemMediaTransportControlsButton::Previous {
                                let on_arrow_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*top_ui).klass() }, c"OnClickSetListArrow", 1);
                                if on_arrow_addr != 0 {
                                    let on_arrow: extern "C" fn(*mut Il2CppObject, bool) = unsafe { std::mem::transmute(on_arrow_addr) };
                                    on_arrow(top_ui, false);
                                }
                            } else if button == SystemMediaTransportControlsButton::Next {
                                let get_data_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*top_ui).klass() }, c"get_TempSetListPlayingData", 0);
                                if get_data_addr != 0 {
                                    let get_data: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(get_data_addr) };
                                    let data = get_data(top_ui);
                                    if !data.is_null() {
                                        let is_playing_field = crate::il2cpp::symbols::get_field_from_name(unsafe { (*data).klass() }, c"IsPlaying");
                                        if !is_playing_field.is_null() {
                                            let is_playing = crate::il2cpp::symbols::get_field_value::<bool>(data, is_playing_field);
                                            if is_playing {
                                                let on_arrow_addr = crate::il2cpp::symbols::get_method_addr_cached(unsafe { (*top_ui).klass() }, c"OnClickSetListArrow", 1);
                                                if on_arrow_addr != 0 {
                                                    let on_arrow: extern "C" fn(*mut Il2CppObject, bool) = unsafe { std::mem::transmute(on_arrow_addr) };
                                                    on_arrow(top_ui, true);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
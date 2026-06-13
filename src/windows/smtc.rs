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
    core::{game::Region, gui::IS_LIVE_SCENE, Hachimi},
    il2cpp::{
        ext::{Il2CppStringExt, StringExt, Il2CppObjectExt},
        hook::{
            UnityEngine_CoreModule::{Behaviour, Texture2D, RenderTexture, Graphics, Texture, SceneManager, Scene},
            UnityEngine_AssetBundleModule::AssetBundle,
            UnityEngine_ImageConversionModule::ImageConversion,
            umamusume::{
                AssetManager, Director::{self, LiveLoadSettings}, HomeViewController, HubViewControllerBase,
                JukeboxBgmSelector, JukeboxHomeTopUI, LiveViewController,
                MasterJukeboxSetlistMusicData, WorkDataManager, WorkJukeboxData,
                SceneManager as UmaSceneManager, TempData::JukeboxSetListPlayingData
            }
        },
        symbols::{get_method_cached, get_type_object_for_class, Thread},
        types::*
    }
};
use crate::il2cpp::sql::get_master_text;

static SMTC_INSTANCE: Lazy<Mutex<Option<SystemMediaTransportControls>>> = Lazy::new(|| Mutex::new(None));
static mut CURRENT_SCENE_HANDLE: i32 = -1;
static mut CURRENT_MUSIC_ID: i32 = -1;
static PENDING_BUTTON: AtomicU32 = AtomicU32::new(u32::MAX);
static mut IS_HOME_SCENE: bool = false;
static mut LAST_UPDATE: Option<std::time::Instant> = None;

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
    if !Hachimi::instance().config.load().windows.enable_smtc {
        return;
    }
    let mut smtc_guard = SMTC_INSTANCE.lock().unwrap();
    if smtc_guard.is_none() {
        unsafe {
            let hwnd = TASKBAR_HWND;
            if hwnd.0.is_null() { return; }

            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
            let title = if Hachimi::instance().game.region == Region::Japan {
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
            IS_LIVE_SCENE.store(name == "Live", Ordering::Release);
            IS_HOME_SCENE = name == "Home";
        }

        if name != "Live" && !IS_LIVE_SCENE.load(Ordering::Acquire) {
            unsafe { CURRENT_MUSIC_ID = -1; }
        }

        let is_live = IS_LIVE_SCENE.load(Ordering::Acquire);
        if name == "Live" || is_live {
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

    // throttle per-frame updates to ~1Hz
    let now = std::time::Instant::now();
    let should_update = unsafe {
        LAST_UPDATE.map_or(true, |last| now.duration_since(last).as_millis() >= 1000)
    };

    if IS_LIVE_SCENE.load(Ordering::Acquire) && should_update {
        unsafe { LAST_UPDATE = Some(now); }

        let current_music_id = unsafe { CURRENT_MUSIC_ID };
        if let Some(live_music_id) = get_live_music_id() {
            if live_music_id != current_music_id {
                update_metadata(smtc, live_music_id);
            }
        }

        update_live_playback(smtc);
    } else if unsafe { IS_HOME_SCENE } && should_update {
        unsafe { LAST_UPDATE = Some(now); }
        update_metadata_home(smtc);
    }
}

fn get_live_music_id() -> Option<i32> {
    let director = Director::instance();
    if director.is_null() { return None; }

    let load_settings = Director::get_LoadSettings(director);
    if load_settings.is_null() { return None; }

    let music_id = LiveLoadSettings::get_MusicId(load_settings);
    if music_id == 0 { return None; }

    Some(music_id)
}

fn get_jacket_texture(music_id: i32) -> Option<*mut Il2CppObject> {
    let loader = AssetManager::get_Loader();
    if loader.is_null() { return None; }

    let music_id_str = format!("{:04}", music_id);
    let jacket_name = format!("jacket_icon_m_{}", music_id_str);
    let path = format!("Live/Jacket/{}", jacket_name);

    let asset_handle = AssetManager::LoadAssetHandle(loader, path.to_il2cpp_string(), false);
    if asset_handle.is_null() { return None; }

    let bundle = AssetManager::get_assetBundle(asset_handle);
    if bundle.is_null() { return None; }

    let t2d_type_obj = get_type_object_for_class(Texture2D::class());
    if t2d_type_obj.is_null() { return None; }

    let tex = AssetBundle::LoadAsset_Internal_orig(bundle, jacket_name.to_il2cpp_string(), t2d_type_obj);
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
        if CURRENT_MUSIC_ID == music_id {
            return;
        }
        if let Ok(updater) = smtc.DisplayUpdater() {
            let _ = updater.SetThumbnail(None);
            let _ = updater.Update();
        }
        CURRENT_MUSIC_ID = music_id;
    }

    let _ = smtc.SetIsPlayEnabled(true);
    let _ = smtc.SetIsPauseEnabled(true);

    if let Ok(updater) = smtc.DisplayUpdater() {
        if let Ok(props) = updater.MusicProperties() {
            if let Some(title) = get_master_text(16, music_id) {
                if !title.is_empty() {
                    let _ = props.SetTitle(&HSTRING::from(title));
                }
            }
            if let Some(artist) = get_master_text(17, music_id) {
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
    debug!("[smtc] update_metadata_home called");
    let mut has_set_list = false;
    let hub_vc = get_current_hub_view_child_controller();
    if !hub_vc.is_null() {
        let hub_name = unsafe { std::ffi::CStr::from_ptr((*(*hub_vc).klass()).name) }.to_string_lossy();
        debug!("[smtc] hub_vc name = {}", hub_name);
        if hub_name == "HomeViewController" {
            let top_ui = HomeViewController::GetTopUI(hub_vc, 10);
            if !top_ui.is_null() {
                let data = JukeboxHomeTopUI::get_TempSetListPlayingData(top_ui);
                if !data.is_null() {
                    let is_playing = JukeboxSetListPlayingData::get_IsPlaying(data);
                    if is_playing {
                        has_set_list = true;
                        set_playback_status(smtc, MediaPlaybackStatus::Playing);

                        let set_list_index = JukeboxSetListPlayingData::get_SetListIndex(data);
                        let music_list_count = JukeboxSetListPlayingData::GetMusicListCount(data);

                        let _ = smtc.SetIsPreviousEnabled(set_list_index > 0);
                        let _ = smtc.SetIsNextEnabled(set_list_index < music_list_count - 1);

                        let music_data = JukeboxSetListPlayingData::GetMasterSetListMusicData(data);
                        if !music_data.is_null() {
                            let music_id = MasterJukeboxSetlistMusicData::JukeboxSetlistMusicData::get_MusicId(music_data);
                            if music_id != 0 {
                                unsafe { LAST_HOME_MUSIC_ID = music_id; }
                                update_metadata(smtc, music_id);
                            }
                        }
                    }
                }
            }
        }
    }

    if !has_set_list {
        let instance = WorkDataManager::instance();
        if instance.is_null() { return; }

        let jukebox_result = microseh::try_seh(|| WorkDataManager::get_JukeboxData(instance));
        let jukebox = match jukebox_result {
            Ok(j) => j,
            Err(_) => {
                warn!("[smtc] Exception in get_JukeboxData, skipping");
                return;
            }
        };

        let music_id = WorkJukeboxData::GetCurrentBgmMusicId(jukebox);
        debug!("[smtc] music_id = {}", music_id);

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
            let state = LiveViewController::get__state(vc);
            let status = if state == 0 { MediaPlaybackStatus::Playing } else { MediaPlaybackStatus::Paused };
            set_playback_status(smtc, status);
            return;
        }
    }

    let instance = Director::instance();
    if instance.is_null() { return; }

    let paused = Director::IsPauseLive(instance);
    let status = if paused { MediaPlaybackStatus::Paused } else { MediaPlaybackStatus::Playing };

    set_playback_status(smtc, status);
}

fn get_current_view_controller() -> *mut Il2CppObject {
    let instance = UmaSceneManager::instance();
    if instance.is_null() { return std::ptr::null_mut(); }

    UmaSceneManager::GetCurrentViewController(instance)
}

fn get_current_hub_view_child_controller() -> *mut Il2CppObject {
    debug!("[smtc] get_current_hub_view_child_controller: getting vc");
    let vc = get_current_view_controller();
    debug!("[smtc] get_current_hub_view_child_controller: vc = {:?}", vc);
    if vc.is_null() { return std::ptr::null_mut(); }

    debug!("[smtc] getting klass");
    let klass = unsafe { (*vc).klass() };
    debug!("[smtc] klass = {:?}", klass);
    let parent_klass = unsafe { (*klass).parent };
    debug!("[smtc] parent_klass = {:?}", parent_klass);
    if parent_klass.is_null() { return std::ptr::null_mut(); }

    debug!("[smtc] getting parent_name");
    let parent_name = unsafe { std::ffi::CStr::from_ptr((*parent_klass).name) }.to_string_lossy();
    debug!("[smtc] parent_name = {}", parent_name);
    if parent_name != "HubViewControllerBase" { return std::ptr::null_mut(); }

    debug!("[smtc] calling get_ChildCurrentController");
    HubViewControllerBase::get_ChildCurrentController(vc)
}

fn handle_button_pressed(args: &SystemMediaTransportControlsButtonPressedEventArgs) {
    if let Ok(button) = args.Button() {
        PENDING_BUTTON.store(button.0 as u32, Ordering::Relaxed);
        Thread::main_thread().schedule(move || {
            let button_val = PENDING_BUTTON.swap(u32::MAX, Ordering::Relaxed);
            if button_val == u32::MAX { return; }
            let button = SystemMediaTransportControlsButton(button_val as i32);

            let vc = get_current_view_controller();
            if !vc.is_null() {
                let name = unsafe { std::ffi::CStr::from_ptr((*(*vc).klass()).name) }.to_string_lossy();
                if name == "LiveViewController" {
                    if button == SystemMediaTransportControlsButton::Pause {
                        LiveViewController::PauseLive(vc);
                    } else if button == SystemMediaTransportControlsButton::Play {
                        LiveViewController::ResumeLive(vc);
                    } else if button == SystemMediaTransportControlsButton::Next {
                        let coroutine = LiveViewController::SkipLive(vc);
                        if !coroutine.is_null() {
                            let view_base = LiveViewController::GetViewBase(vc);
                            if !view_base.is_null() {
                                Behaviour::StartCoroutine(view_base, coroutine);
                            }
                        }
                    }
                }
            }

            let hub_vc = get_current_hub_view_child_controller();
            debug!("[smtc] hub_vc = {:?}", hub_vc);
            if !hub_vc.is_null() {
                let hub_name = unsafe { std::ffi::CStr::from_ptr((*(*hub_vc).klass()).name) }.to_string_lossy();
                debug!("[smtc] hub_vc name = {}", hub_name);
                if hub_name == "HomeViewController" {
                    // 10 = HomeTopState.Jukebox
                    let top_ui = HomeViewController::GetTopUI(hub_vc, 10);
                    if !top_ui.is_null() {
                        if button == SystemMediaTransportControlsButton::Play {
                            let data = JukeboxHomeTopUI::get_TempSetListPlayingData(top_ui);
                            let mut is_set_list = false;
                            if !data.is_null() {
                                let set_list_id = JukeboxSetListPlayingData::get_SetListId(data);
                                if set_list_id > 0 {
                                    is_set_list = true;
                                    let selector = JukeboxHomeTopUI::get_JukeboxBgmSelector(top_ui);
                                    if !selector.is_null() {
                                        JukeboxBgmSelector::PlayCoroutinePlaySetList(selector, true, 0.0, true);
                                    }
                                }
                            }
                            if !is_set_list {
                                let JukeboxHomeTopUI_class = JukeboxHomeTopUI::class();
                                if get_method_cached(JukeboxHomeTopUI_class, c"SetPlayMusicFlag", 1).is_ok() {
                                    JukeboxHomeTopUI::SetPlayMusicFlag(top_ui, true);
                                } else {
                                    JukeboxHomeTopUI::PlayRequestSong(top_ui);
                                }
                            }
                        } else if button == SystemMediaTransportControlsButton::Pause {
                            JukeboxHomeTopUI::SetPlayMusicFlag(top_ui, false);
                        } else if button == SystemMediaTransportControlsButton::Previous {
                            JukeboxHomeTopUI::OnClickSetListArrow(top_ui, false);
                        } else if button == SystemMediaTransportControlsButton::Next {
                            let data = JukeboxHomeTopUI::get_TempSetListPlayingData(top_ui);
                            if !data.is_null() {
                                let is_playing = JukeboxSetListPlayingData::get_IsPlaying(data);
                                if is_playing {
                                    JukeboxHomeTopUI::OnClickSetListArrow(top_ui, true);
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}

pub fn unregister() {
    let mut smtc_guard = SMTC_INSTANCE.lock().unwrap();
    if let Some(smtc) = smtc_guard.take() {
        let _ = smtc.SetIsEnabled(false);
        unsafe {
            CURRENT_MUSIC_ID = -1;
            CURRENT_SCENE_HANDLE = -1;
            LAST_STATUS = MediaPlaybackStatus::Closed;
        }
        info!("SMTC unregistered");
    }
}

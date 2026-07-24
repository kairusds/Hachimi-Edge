use crate::{core::{Hachimi, game::Region}, il2cpp::{symbols::{IEnumerator, MoveNextFn, SingletonLike, get_method_addr}, types::*}};
#[cfg(target_os = "windows")]
use crate::core::{free_camera::{self, CameraScene}, live_utils};
#[cfg(target_os = "windows")]
use super::Director;
// use std::sync::atomic::{AtomicBool, Ordering};

// pub static GAME_INITIALIZED: AtomicBool = AtomicBool::new(false);

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

static mut SOFTWARERESET_ADDR: usize = 0;
impl_addr_wrapper_fn!(SoftwareReset, SOFTWARERESET_ADDR, (), this: *mut Il2CppObject);

#[cfg(target_os = "windows")]
type GameSystemUpdateFn = extern "C" fn(this: *mut Il2CppObject);
#[cfg(target_os = "windows")]
fn apply_free_camera_live_pause_request() {
    if !free_camera::take_toggle_live_pause_request() {
        return;
    }
    live_utils::toggle_live_pause();
}

#[cfg(target_os = "windows")]
extern "C" fn GameSystem_Update(this: *mut Il2CppObject) {
    apply_free_camera_live_pause_request();

    // Live and race normally tick from their camera LateUpdate hooks. Keep the
    // global update path only as a fallback while LiveTimelineControl is paused.
    if Director::is_live_paused() && free_camera::scene() == CameraScene::Live {
        free_camera::tick();
        apply_free_camera_live_pause_request();
    }
    get_orig_fn!(GameSystem_Update, GameSystemUpdateFn)(this);
}

#[cfg(target_os = "windows")]
type GameSystemLateUpdateFn = extern "C" fn(this: *mut Il2CppObject);
#[cfg(target_os = "windows")]
extern "C" fn GameSystem_LateUpdate(this: *mut Il2CppObject) {
    get_orig_fn!(GameSystem_LateUpdate, GameSystemLateUpdateFn)(this);
    Director::apply_paused_free_camera();
}

// good hook for initializing values i guess
pub fn on_game_initialized() {
    Hachimi::instance().init_character_data();
    // GAME_INITIALIZED.store(true, Ordering::Relaxed);
    Hachimi::instance().init_skill_info();
    #[cfg(target_os = "android")]
    crate::android::utils::set_audio_capture_policy_all();
    #[cfg(target_os = "windows")]
    super::UIManager::apply_ui_scale();

    // Invoke plugin callbacks
    let hachimi = Hachimi::instance();
    let callbacks = hachimi.plugin_init_callbacks.lock().unwrap();
    for (callback, userdata) in callbacks.iter() {
        let callback_ptr = *callback;
        if callback_ptr == 0 { continue; }
        let callback: unsafe extern "C" fn(*mut std::ffi::c_void) = unsafe { std::mem::transmute(callback_ptr) };
        unsafe { callback(*userdata as *mut std::ffi::c_void); }
    }
}

extern "C" fn InitializeGame_MoveNext(enumerator: *mut Il2CppObject) -> bool {
    let moved = get_orig_fn!(InitializeGame_MoveNext, MoveNextFn)(enumerator);
    if !moved {
        // Game has finished initializing
        on_game_initialized();
    }
    moved
}

fn InitializeGameCommon(enumerator: IEnumerator) -> IEnumerator {
    if Hachimi::instance().config.load().ui_scale == 1.0 { return enumerator; }

    if let Err(e) = enumerator.hook_move_next(InitializeGame_MoveNext) {
        error!("Failed to hook InitializeGame enumerator: {}", e);
    }

    enumerator
}

type InitializeGameJpFn = extern "C" fn(this: *mut Il2CppObject, on_complete_initialize_ui: *mut Il2CppObject) -> IEnumerator;
extern "C" fn InitializeGameJp(this: *mut Il2CppObject, on_complete_initialize_ui: *mut Il2CppObject) -> IEnumerator {
    let enumerator = get_orig_fn!(InitializeGameJp, InitializeGameJpFn)(this, on_complete_initialize_ui);
    InitializeGameCommon(enumerator)
}

type InitializeGameOtherFn = extern "C" fn(this: *mut Il2CppObject) -> IEnumerator;
extern "C" fn InitializeGameOther(this: *mut Il2CppObject) -> IEnumerator {
    let enumerator = get_orig_fn!(InitializeGameOther, InitializeGameOtherFn)(this);
    InitializeGameCommon(enumerator)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, GameSystem);

    if Hachimi::instance().game.region == Region::Japan {
        let InitializeGame_addr = get_method_addr(GameSystem, c"InitializeGame", 1);
        new_hook!(InitializeGame_addr, InitializeGameJp);
    }
    else {
        let InitializeGame_addr = get_method_addr(GameSystem, c"InitializeGame", 0);
        new_hook!(InitializeGame_addr, InitializeGameOther);
    }

    unsafe {
        CLASS = GameSystem;
        SOFTWARERESET_ADDR = get_method_addr(GameSystem, c"SoftwareReset", 0);
    }

    #[cfg(target_os = "windows")]
    {
        let GameSystem_Update_addr = get_method_addr(GameSystem, c"Update", 0);
        new_hook!(GameSystem_Update_addr, GameSystem_Update);
        let GameSystem_LateUpdate_addr = get_method_addr(GameSystem, c"LateUpdate", 0);
        new_hook!(GameSystem_LateUpdate_addr, GameSystem_LateUpdate);
    }
}

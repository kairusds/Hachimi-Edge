use std::{
    cell::Cell,
    sync::{Mutex, atomic::{AtomicBool, Ordering}},
    time::Instant,
};

use once_cell::sync::Lazy;
use rust_i18n::t;
use serde::{Deserialize, Serialize};

use crate::{core::{gui, Hachimi}, il2cpp::types::{Quaternion_t, Vector3_t}};

const LOOK_RADIUS: f32 = 5.0;
const OVERLAY_FADE_IN: f32 = 0.18;
const OVERLAY_HOLD: f32 = 1.6;
const OVERLAY_FADE_OUT: f32 = 0.35;
const UNSUPPORTED_LIVE_MUSIC_ID: i32 = 1157;

pub const LIVE_POSITION_CHOICES: &[(&str, i32)] = &[
    ("Place01", 0x1),
    ("Place02", 0x2),
    ("Place03", 0x4),
    ("Place04", 0x8),
    ("Place05", 0x10),
    ("Place06", 0x20),
    ("Place07", 0x40),
    ("Place08", 0x80),
    ("Place09", 0x100),
    ("Place10", 0x200),
    ("Place11", 0x400),
    ("Place12", 0x800),
    ("Place13", 0x1000),
    ("Place14", 0x2000),
    ("Place15", 0x4000),
    ("Place16", 0x8000),
    ("Place17", 0x10000),
    ("Place18", 0x20000),
    ("Place19", 0x40000),
    ("Place20", 0x80000),
    ("Center", 0x1),
    ("Left", 0x2),
    ("Right", 0x4),
    ("Side", 0x6),
    ("Back", 0xffff8),
    ("Other", 0xffffe),
    ("All", 0xfffff),
];

pub const LIVE_PART_CHOICES: &[(&str, i32)] = &[
    ("Face", 0x0),
    ("Waist", 0x1),
    ("LeftHandWrist", 0x2),
    ("RightHandAttach", 0x3),
    ("Chest", 0x4),
    ("Foot", 0x5),
    ("InitFaceHeight", 0x6),
    ("InitWaistHeight", 0x7),
    ("InitChestHeight", 0x8),
    ("RightHandWrist", 0x9),
    ("LeftHandAttach", 0xa),
    ("ConstFaceHeight", 0xb),
    ("ConstChestHeight", 0xc),
    ("ConstWaistHeight", 0xd),
    ("ConstFootHeight", 0xe),
    ("Position", 0xf),
    ("PositionWithoutOffset", 0x10),
    ("InitialHeightFace", 0x11),
    ("InitialHeightChest", 0x12),
    ("InitialHeightWaist", 0x13),
    ("StartFrameFace", 0x14),
    ("Max", 0x15),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum FreeCameraMode {
    Free,
    FirstPerson,
    SelfieStick,
}

impl Default for FreeCameraMode {
    fn default() -> Self {
        Self::Free
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
pub struct Vec3Config {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3Config {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl Default for Vec3Config {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FreeCameraKeybinds {
    pub move_forward: u16,
    pub move_back: u16,
    pub move_left: u16,
    pub move_right: u16,
    pub move_down: u16,
    pub move_up: u16,
    pub look_up: u16,
    pub look_down: u16,
    pub look_left: u16,
    pub look_right: u16,
    pub fov_increase: u16,
    pub fov_decrease: u16,
    pub follow_offset_up: u16,
    pub follow_offset_down: u16,
    pub follow_offset_left: u16,
    pub follow_offset_right: u16,
    pub target_previous: u16,
    pub target_next: u16,
    pub part_previous: u16,
    pub part_next: u16,
    pub reset: u16,
    pub cycle_mode: u16,
    pub reverse: u16,
}

#[cfg(target_os = "windows")]
impl Default for FreeCameraKeybinds {
    fn default() -> Self {
        Self {
            move_forward: 0x57,      // W
            move_back: 0x53,         // S
            move_left: 0x41,         // A
            move_right: 0x44,        // D
            move_down: 0xa2,         // Left Ctrl
            move_up: 0x20,           // Space
            look_up: 0x26,           // Up
            look_down: 0x28,         // Down
            look_left: 0x25,         // Left
            look_right: 0x27,        // Right
            fov_increase: 0x51,      // Q
            fov_decrease: 0x45,      // E
            follow_offset_up: 0x49,  // I
            follow_offset_down: 0x4b,// K
            follow_offset_left: 0x4a,// J
            follow_offset_right: 0x4c,// L
            target_previous: 0xdb,   // [
            target_next: 0xdd,       // ]
            part_previous: 0xba,     // ;
            part_next: 0xde,         // '
            reset: 0x52,             // R
            cycle_mode: 0x46,        // F
            reverse: 0x56,           // V
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FreeCameraConfig {
    pub enabled: bool,
    pub remove_camera_effects: bool,
    pub live_remove_screen_effects: bool,
    pub live_disable_character_teleport: bool,
    pub live_force_all_characters_visible: bool,
    pub show_overlay: bool,
    pub selfie_use_head_transform: bool,
    pub live_selfie_horizontal_stabilization: f32,
    pub live_selfie_vertical_stabilization: f32,
    pub mode: FreeCameraMode,
    pub live_move_step: f32,
    pub race_move_step: f32,
    pub look_step: f32,
    pub mouse_speed: f32,
    pub live_fov: f32,
    pub race_fov: f32,
    pub live_target_position_index: i32,
    pub live_target_part_index: i32,
    pub live_follow_offset: Vec3Config,
    pub live_follow_lookat_offset: Vec3Config,
    pub live_follow_smooth: bool,
    pub live_follow_smooth_lookat_step: f32,
    pub live_follow_smooth_pos_step: f32,
    pub live_first_person_offset: Vec3Config,
    pub race_target_index: i32,
    pub race_follow_offset: Vec3Config,
    pub race_follow_distance: f32,
    pub race_first_person_lookat_offset: Vec3Config,
    pub gamepad_deadzone: f32,
    pub gamepad_move_speed: f32,
    pub gamepad_look_speed: f32,

    #[cfg(target_os = "windows")]
    pub keybinds: FreeCameraKeybinds,
}

impl Default for FreeCameraConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            remove_camera_effects: true,
            live_remove_screen_effects: false,
            live_disable_character_teleport: false,
            live_force_all_characters_visible: false,
            show_overlay: true,
            selfie_use_head_transform: false,
            live_selfie_horizontal_stabilization: 0.0,
            live_selfie_vertical_stabilization: 0.0,
            mode: FreeCameraMode::Free,
            live_move_step: 0.2,
            race_move_step: 0.25,
            look_step: 1.0,
            mouse_speed: 10.0,
            live_fov: 60.0,
            race_fov: 60.0,
            live_target_position_index: 0,
            live_target_part_index: 0,
            live_follow_offset: Vec3Config::new(0.0, 0.0, -2.0),
            live_follow_lookat_offset: Vec3Config::default(),
            live_follow_smooth: false,
            live_follow_smooth_lookat_step: 0.35,
            live_follow_smooth_pos_step: 0.25,
            live_first_person_offset: Vec3Config::new(0.0, 0.075, 0.015),
            race_target_index: -1,
            race_follow_offset: Vec3Config::new(0.0, 2.5, -8.0),
            race_follow_distance: 0.0,
            race_first_person_lookat_offset: Vec3Config::default(),
            gamepad_deadzone: 0.18,
            gamepad_move_speed: 1.0,
            gamepad_look_speed: 1.0,

            #[cfg(target_os = "windows")]
            keybinds: FreeCameraKeybinds::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraScene {
    #[default]
    None,
    Live,
    Race,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GamepadAxes {
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn from_config(value: Vec3Config) -> Self {
        Self::new(value.x, value.y, value.z)
    }

    fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    fn normalized(self) -> Self {
        let len = self.len();
        if len <= f32::EPSILON {
            Self::default()
        }
        else {
            self * (1.0 / len)
        }
    }

    fn lerp(self, target: Self, amount: f32) -> Self {
        self + (target - self) * amount
    }

    fn to_vector3(self) -> Vector3_t {
        Vector3_t { x: self.x, y: self.y, z: self.z }
    }
}

impl From<Vector3_t> for Vec3 {
    fn from(value: Vector3_t) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Self) -> Self::Output {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Quat {
    w: f32,
    x: f32,
    y: f32,
    z: f32,
}

impl Quat {
    fn from_quaternion(value: Quaternion_t) -> Self {
        Self {
            w: value.w,
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }

    fn to_quaternion(self) -> Quaternion_t {
        Quaternion_t {
            w: self.w,
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }

    fn conjugate(self) -> Self {
        Self { w: self.w, x: -self.x, y: -self.y, z: -self.z }
    }

    fn dot(self, rhs: Self) -> f32 {
        self.w * rhs.w + self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn normalized(self) -> Self {
        let len = (self.dot(self)).sqrt();
        if len <= f32::EPSILON {
            return Self { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };
        }
        Self {
            w: self.w / len,
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
        }
    }

    fn rotate_axis(self, angle_degrees: f32, axis: Vec3) -> Self {
        let angle = angle_degrees.to_radians() * 0.5;
        let axis = axis.normalized();
        let q = Quat {
            w: angle.cos(),
            x: axis.x * angle.sin(),
            y: axis.y * angle.sin(),
            z: axis.z * angle.sin(),
        };
        (self * q).normalized()
    }

    fn rotate_vec(self, vec: Vec3) -> Vec3 {
        let p = Quat { w: 0.0, x: vec.x, y: vec.y, z: vec.z };
        let out = self * p * self.conjugate();
        Vec3::new(out.x, out.y, out.z)
    }

    fn slerp(self, rhs: Self, t: f32) -> Self {
        let mut other = rhs;
        let mut dot = self.dot(other);
        if dot < 0.0 {
            dot = -dot;
            other = Quat {
                w: -other.w,
                x: -other.x,
                y: -other.y,
                z: -other.z,
            };
        }

        if dot > 0.95 {
            return Quat {
                w: self.w + t * (other.w - self.w),
                x: self.x + t * (other.x - self.x),
                y: self.y + t * (other.y - self.y),
                z: self.z + t * (other.z - self.z),
            }.normalized();
        }

        let angle = dot.clamp(-1.0, 1.0).acos();
        let sin_angle = angle.sin();
        if sin_angle.abs() <= f32::EPSILON {
            return self;
        }
        let sin_a = ((1.0 - t) * angle).sin() / sin_angle;
        let sin_b = (t * angle).sin() / sin_angle;
        Quat {
            w: self.w * sin_a + other.w * sin_b,
            x: self.x * sin_a + other.x * sin_b,
            y: self.y * sin_a + other.y * sin_b,
            z: self.z * sin_a + other.z * sin_b,
        }.normalized()
    }
}

impl std::ops::Mul for Quat {
    type Output = Quat;

    fn mul(self, rhs: Self) -> Self::Output {
        Quat {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct KeyState {
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
    down: bool,
    up: bool,
    look_up: bool,
    look_down: bool,
    look_left: bool,
    look_right: bool,
    fov_increase: bool,
    fov_decrease: bool,
    follow_offset_up: bool,
    follow_offset_down: bool,
    follow_offset_left: bool,
    follow_offset_right: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct GamepadState {
    axes: GamepadAxes,
    lb: bool,
    rb: bool,
    last_buttons: u16,
}

#[derive(Debug)]
struct FreeCameraState {
    scene: CameraScene,
    mode: FreeCameraMode,
    camera_pos: Vec3,
    camera_look_at: Vec3,
    camera_rotation: Option<Quat>,
    yaw: f32,
    pitch: f32,
    live_fov: f32,
    race_fov: f32,
    live_target_position_index: i32,
    live_target_part_index: i32,
    live_follow_offset: Vec3,
    live_follow_lookat_offset: Vec3,
    live_first_person_offset: Vec3,
    live_follow_target: Option<Vec3>,
    live_follow_position_target: Option<Vec3>,
    live_follow_precise_target: bool,
    live_follow_timeline_updated: bool,
    live_head_part_target: Option<Vec3>,
    live_selfie_camera_offset: Option<Vec3>,
    live_selfie_look_offset: Option<Vec3>,
    live_selfie_head_pos: Option<Vec3>,
    live_selfie_head_forward: Option<Vec3>,
    live_selfie_last_head_pos: Option<Vec3>,
    live_selfie_stabilized_target: Option<Vec3>,
    race_target_index: i32,
    race_follow_offset: Vec3,
    race_follow_distance: f32,
    race_first_person_lookat_offset: Vec3,
    race_target_last: Vec3,
    race_target: Vec3,
    race_target_rot: Quat,
    race_target_seen: bool,
    key_state: KeyState,
    gamepad: GamepadState,
    right_mouse_down: bool,
    last_mouse_pos: Option<(i32, i32)>,
    last_tick: Instant,
    last_enabled: bool,
    last_config_mode: FreeCameraMode,
    last_overlay_mode: FreeCameraMode,
}

struct OverlayMessage {
    content: String,
    created_at: Instant,
}

impl FreeCameraState {
    fn new() -> Self {
        let config = FreeCameraConfig::default();
        let mut state = Self {
            scene: CameraScene::None,
            mode: config.mode,
            camera_pos: Vec3::default(),
            camera_look_at: Vec3::default(),
            camera_rotation: None,
            yaw: 0.0,
            pitch: 0.0,
            live_fov: config.live_fov,
            race_fov: config.race_fov,
            live_target_position_index: 0,
            live_target_part_index: 0,
            live_follow_offset: Vec3::default(),
            live_follow_lookat_offset: Vec3::default(),
            live_first_person_offset: Vec3::default(),
            live_follow_target: None,
            live_follow_position_target: None,
            live_follow_precise_target: false,
            live_follow_timeline_updated: false,
            live_head_part_target: None,
            live_selfie_camera_offset: None,
            live_selfie_look_offset: None,
            live_selfie_head_pos: None,
            live_selfie_head_forward: None,
            live_selfie_last_head_pos: None,
            live_selfie_stabilized_target: None,
            race_target_index: -1,
            race_follow_offset: Vec3::default(),
            race_follow_distance: 0.0,
            race_first_person_lookat_offset: Vec3::default(),
            race_target_last: Vec3::default(),
            race_target: Vec3::default(),
            race_target_rot: Quat { w: 1.0, x: 0.0, y: 0.0, z: 0.0 },
            race_target_seen: false,
            key_state: KeyState::default(),
            gamepad: GamepadState::default(),
            right_mouse_down: false,
            last_mouse_pos: None,
            last_tick: Instant::now(),
            last_enabled: false,
            last_config_mode: config.mode,
            last_overlay_mode: config.mode,
        };
        state.reset_with_config(&config);
        state
    }

    fn reset_with_config(&mut self, config: &FreeCameraConfig) {
        self.mode = config.mode;
        self.last_config_mode = config.mode;
        self.last_overlay_mode = config.mode;
        self.live_fov = config.live_fov;
        self.live_target_position_index =
            config.live_target_position_index.clamp(0, LIVE_POSITION_CHOICES.len() as i32 - 1);
        self.live_target_part_index =
            config.live_target_part_index.clamp(0, LIVE_PART_CHOICES.len() as i32 - 1);
        self.live_follow_offset = Vec3::from_config(config.live_follow_offset);
        self.live_follow_lookat_offset = Vec3::from_config(config.live_follow_lookat_offset);
        self.live_first_person_offset = Vec3::from_config(config.live_first_person_offset);
        self.live_follow_target = None;
        self.live_follow_position_target = None;
        self.live_follow_precise_target = false;
        self.live_follow_timeline_updated = false;
        self.live_head_part_target = None;
        self.live_selfie_camera_offset = None;
        self.live_selfie_look_offset = None;
        self.live_selfie_head_pos = None;
        self.live_selfie_head_forward = None;
        self.live_selfie_last_head_pos = None;
        self.live_selfie_stabilized_target = None;
        self.race_target_index = config.race_target_index;
        self.race_follow_offset = Vec3::from_config(config.race_follow_offset);
        self.race_follow_distance = config.race_follow_distance;
        self.race_first_person_lookat_offset = Vec3::from_config(config.race_first_person_lookat_offset);

        if config.selfie_use_head_transform {
            self.live_follow_offset = Vec3::new(0.0, 0.0, -2.0);
            self.live_follow_lookat_offset = Vec3::default();
            self.race_follow_offset = Vec3::new(0.0, 0.0, -2.0);
            self.race_follow_distance = 0.0;
            self.race_first_person_lookat_offset = Vec3::default();
        }

        self.race_target_seen = false;
        self.camera_rotation = None;
        self.key_state = KeyState::default();
        self.right_mouse_down = false;
        self.last_mouse_pos = None;

        if self.scene == CameraScene::Race {
            self.camera_pos = Vec3::new(-51.72, 7.91, 108.57);
        }
        else {
            self.camera_pos = Vec3::new(0.093706, 0.467159, 9.588791);
        }
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.update_look_from_angles();
    }

    fn reset_current_mode_camera(&mut self, config: &FreeCameraConfig) {
        self.live_fov = config.live_fov;
        self.race_fov = config.race_fov;
        self.camera_rotation = None;
        self.key_state = KeyState::default();
        self.right_mouse_down = false;
        self.last_mouse_pos = None;

        match self.mode {
            FreeCameraMode::Free => {
                if self.scene == CameraScene::Race {
                    self.camera_pos = Vec3::new(-51.72, 7.91, 108.57);
                }
                else {
                    self.camera_pos = Vec3::new(0.093706, 0.467159, 9.588791);
                }
                self.yaw = 0.0;
                self.pitch = 0.0;
                self.update_look_from_angles();
            },
            FreeCameraMode::SelfieStick => {
                self.live_follow_target = None;
                self.live_follow_position_target = None;
                self.live_follow_precise_target = false;
                self.live_follow_timeline_updated = false;
                self.live_head_part_target = None;
                self.live_selfie_camera_offset = None;
                self.live_selfie_look_offset = None;
                self.live_selfie_head_pos = None;
                self.live_selfie_head_forward = None;
                self.live_selfie_last_head_pos = None;
                self.live_selfie_stabilized_target = None;
                if self.scene == CameraScene::Race {
                    if config.selfie_use_head_transform {
                        self.race_follow_offset = Vec3::new(0.0, 0.0, -2.0);
                        self.race_follow_distance = 0.0;
                    }
                    else {
                        self.race_follow_offset = Vec3::from_config(config.race_follow_offset);
                        self.race_follow_distance = config.race_follow_distance;
                    }
                    self.race_first_person_lookat_offset =
                        Vec3::from_config(config.race_first_person_lookat_offset);
                }
                else {
                    self.live_follow_offset = if config.selfie_use_head_transform {
                        Vec3::new(0.0, 0.0, -2.0)
                    }
                    else {
                        Vec3::from_config(config.live_follow_offset)
                    };
                    self.live_follow_lookat_offset = Vec3::from_config(config.live_follow_lookat_offset);
                }
            },
            FreeCameraMode::FirstPerson => {
                if self.scene == CameraScene::Live {
                    self.live_first_person_offset = Vec3::from_config(config.live_first_person_offset);
                }
                else {
                    self.race_first_person_lookat_offset =
                        Vec3::from_config(config.race_first_person_lookat_offset);
                }
            },
        }
    }

    fn set_scene(&mut self, scene: CameraScene, config: &FreeCameraConfig) {
        if self.scene != scene {
            self.scene = scene;
            self.reset_with_config(config);
        }
    }

    fn update_look_from_angles(&mut self) {
        let yaw = self.yaw.to_radians();
        let pitch = self.pitch.to_radians();
        let forward = Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        self.camera_look_at = self.camera_pos + forward * LOOK_RADIUS;
        self.camera_rotation = None;
    }
}

static STATE: Lazy<Mutex<FreeCameraState>> = Lazy::new(|| Mutex::new(FreeCameraState::new()));
static OVERLAY_MESSAGE: Lazy<Mutex<Option<OverlayMessage>>> = Lazy::new(|| Mutex::new(None));
static RELOAD_CONFIG_REQUESTED: AtomicBool = AtomicBool::new(false);
static LIVE_UNSUPPORTED: AtomicBool = AtomicBool::new(false);
static TOGGLE_LIVE_PAUSE_REQUESTED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static LIVE_SECONDARY_CAMERA_UPDATE_DEPTH: Cell<u32> = const { Cell::new(0) };
}

pub struct LiveSecondaryCameraUpdateGuard;

impl Drop for LiveSecondaryCameraUpdateGuard {
    fn drop(&mut self) {
        LIVE_SECONDARY_CAMERA_UPDATE_DEPTH.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

pub fn begin_live_secondary_camera_update() -> LiveSecondaryCameraUpdateGuard {
    LIVE_SECONDARY_CAMERA_UPDATE_DEPTH.with(|depth| {
        depth.set(depth.get().saturating_add(1));
    });
    LiveSecondaryCameraUpdateGuard
}

pub fn is_live_secondary_camera_update() -> bool {
    LIVE_SECONDARY_CAMERA_UPDATE_DEPTH.with(|depth| depth.get() != 0)
}

pub fn reload_runtime_config() {
    RELOAD_CONFIG_REQUESTED.store(true, Ordering::Release);
}

pub fn is_enabled() -> bool {
    Hachimi::instance().config.load().free_camera.enabled &&
        !LIVE_UNSUPPORTED.load(Ordering::Acquire)
}

pub fn set_live_music_id(music_id: i32) {
    let unsupported = music_id == UNSUPPORTED_LIVE_MUSIC_ID;
    let was_unsupported = LIVE_UNSUPPORTED.swap(unsupported, Ordering::AcqRel);

    if !unsupported {
        return;
    }

    let config = Hachimi::instance().config.load();
    let mut state = STATE.lock().unwrap();
    if state.scene == CameraScene::Live {
        state.scene = CameraScene::None;
        state.reset_with_config(&config.free_camera);
    }
    drop(state);
    *OVERLAY_MESSAGE.lock().unwrap() = None;

    if !was_unsupported && config.free_camera.enabled {
        gui::request_notification(gui::NotificationRequest::Custom(
            t!("notification.free_camera_unavailable_live_1157").into_owned(),
        ));
    }
}

pub fn is_game_input_capture_active() -> bool {
    if !is_enabled() {
        return false;
    }

    matches!(STATE.lock().unwrap().scene, CameraScene::Live | CameraScene::Race)
}

pub fn overlay_message() -> Option<(String, f32)> {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || !config.free_camera.show_overlay {
        return None;
    }

    let mut lock = OVERLAY_MESSAGE.lock().unwrap();
    let message = lock.as_ref()?;
    let elapsed = message.created_at.elapsed().as_secs_f32();
    let lifetime = OVERLAY_FADE_IN + OVERLAY_HOLD + OVERLAY_FADE_OUT;
    if elapsed >= lifetime {
        *lock = None;
        return None;
    }

    let alpha = if elapsed < OVERLAY_FADE_IN {
        elapsed / OVERLAY_FADE_IN
    }
    else if elapsed > OVERLAY_FADE_IN + OVERLAY_HOLD {
        1.0 - ((elapsed - OVERLAY_FADE_IN - OVERLAY_HOLD) / OVERLAY_FADE_OUT)
    }
    else {
        1.0
    };

    Some((message.content.clone(), alpha.clamp(0.0, 1.0)))
}

pub fn has_overlay_message() -> bool {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || !config.free_camera.show_overlay {
        return false;
    }

    let lock = OVERLAY_MESSAGE.lock().unwrap();
    let Some(message) = lock.as_ref() else {
        return false;
    };

    message.created_at.elapsed().as_secs_f32() <= OVERLAY_FADE_IN + OVERLAY_HOLD + OVERLAY_FADE_OUT
}

fn set_overlay_message(content: String) {
    if !Hachimi::instance().config.load().free_camera.show_overlay {
        return;
    }

    *OVERLAY_MESSAGE.lock().unwrap() = Some(OverlayMessage {
        content,
        created_at: Instant::now(),
    });
}

fn live_target_label(index: i32) -> String {
    LIVE_POSITION_CHOICES
        .get(index as usize)
        .map(|(name, _)| (*name).to_owned())
        .unwrap_or_else(|| "Unknown".to_owned())
}

fn live_part_label(index: i32) -> String {
    LIVE_PART_CHOICES
        .get(index as usize)
        .map(|(name, _)| (*name).to_owned())
        .unwrap_or_else(|| "Unknown".to_owned())
}

fn race_target_label(index: i32) -> String {
    if index < 0 {
        t!("free_camera.target_auto").into_owned()
    }
    else {
        t!("free_camera.target_gate", index = index + 1).into_owned()
    }
}

fn mode_label(mode: FreeCameraMode) -> String {
    match mode {
        FreeCameraMode::Free => t!("free_camera.mode_free").into_owned(),
        FreeCameraMode::FirstPerson => t!("free_camera.mode_first_person").into_owned(),
        FreeCameraMode::SelfieStick => t!("free_camera.mode_selfie_stick").into_owned(),
    }
}

pub fn scene() -> CameraScene {
    STATE.lock().unwrap().scene
}

pub fn is_scene_enabled(scene: CameraScene) -> bool {
    let config = Hachimi::instance().config.load();
    config.free_camera.enabled &&
        !LIVE_UNSUPPORTED.load(Ordering::Acquire) &&
        STATE.lock().unwrap().scene == scene
}

pub fn mode() -> FreeCameraMode {
    STATE.lock().unwrap().mode
}

pub fn is_live_selfie_stick() -> bool {
    if !is_enabled() {
        return false;
    }

    let state = STATE.lock().unwrap();
    state.scene == CameraScene::Live && state.mode == FreeCameraMode::SelfieStick
}

pub fn is_live_first_person() -> bool {
    if !is_enabled() {
        return false;
    }

    let state = STATE.lock().unwrap();
    state.scene == CameraScene::Live && state.mode == FreeCameraMode::FirstPerson
}

pub fn is_race_first_person() -> bool {
    if !is_enabled() {
        return false;
    }

    let state = STATE.lock().unwrap();
    state.scene == CameraScene::Race && state.mode == FreeCameraMode::FirstPerson
}

pub fn is_live_head_selfie() -> bool {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || !config.free_camera.selfie_use_head_transform {
        return false;
    }

    let state = STATE.lock().unwrap();
    state.scene == CameraScene::Live && state.mode == FreeCameraMode::SelfieStick
}

pub fn is_race_head_selfie() -> bool {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || !config.free_camera.selfie_use_head_transform {
        return false;
    }

    let state = STATE.lock().unwrap();
    state.scene == CameraScene::Race && state.mode == FreeCameraMode::SelfieStick
}

pub fn camera_pos() -> Vector3_t {
    STATE.lock().unwrap().camera_pos.to_vector3()
}

pub fn camera_look_at() -> Vector3_t {
    STATE.lock().unwrap().camera_look_at.to_vector3()
}

pub fn camera_rotation() -> Option<Quaternion_t> {
    STATE.lock().unwrap().camera_rotation.map(|rot| rot.to_quaternion())
}

pub fn fov_for_scene(scene: CameraScene) -> Option<f32> {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return None;
    }

    let state = STATE.lock().unwrap();
    if state.scene != scene {
        return None;
    }

    Some(match scene {
        CameraScene::Live => state.live_fov,
        CameraScene::Race => state.race_fov,
        CameraScene::None => return None,
    })
}

pub fn should_remove_camera_effects() -> bool {
    let config = Hachimi::instance().config.load();
    config.free_camera.enabled &&
        config.free_camera.remove_camera_effects &&
        STATE.lock().unwrap().scene == CameraScene::Live
}

pub fn should_remove_live_screen_effects() -> bool {
    let config = Hachimi::instance().config.load();
    config.free_camera.enabled &&
        config.free_camera.live_remove_screen_effects &&
        !LIVE_UNSUPPORTED.load(Ordering::Acquire) &&
        STATE.lock().unwrap().scene == CameraScene::Live
}

pub fn should_disable_live_character_teleport() -> bool {
    let config = Hachimi::instance().config.load();
    config.free_camera.enabled &&
        config.free_camera.live_disable_character_teleport &&
        STATE.lock().unwrap().scene == CameraScene::Live
}

pub fn should_force_live_characters_visible() -> bool {
    let config = Hachimi::instance().config.load();
    config.free_camera.enabled &&
        config.free_camera.live_force_all_characters_visible &&
        STATE.lock().unwrap().scene == CameraScene::Live
}

pub fn set_live_active() {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || LIVE_UNSUPPORTED.load(Ordering::Acquire) {
        return;
    }

    STATE.lock().unwrap().set_scene(CameraScene::Live, &config.free_camera);
}

pub fn begin_live_director_update() {
    STATE.lock().unwrap().live_follow_timeline_updated = false;
}

pub fn set_race_active() {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return;
    }

    STATE.lock().unwrap().set_scene(CameraScene::Race, &config.free_camera);
}

pub fn end_scene(scene: CameraScene) {
    if scene == CameraScene::Live {
        LIVE_UNSUPPORTED.store(false, Ordering::Release);
        TOGGLE_LIVE_PAUSE_REQUESTED.store(false, Ordering::Release);
    }

    let config = Hachimi::instance().config.load();
    let mut state = STATE.lock().unwrap();
    if state.scene == scene {
        state.scene = CameraScene::None;
        state.reset_with_config(&config.free_camera);
    }
}

pub fn take_toggle_live_pause_request() -> bool {
    TOGGLE_LIVE_PAUSE_REQUESTED.swap(false, Ordering::AcqRel)
}

fn request_toggle_live_pause_locked(state: &FreeCameraState) {
    if state.scene == CameraScene::Live {
        TOGGLE_LIVE_PAUSE_REQUESTED.store(true, Ordering::Release);
    }
}

pub fn live_position_flag() -> i32 {
    let state = STATE.lock().unwrap();
    LIVE_POSITION_CHOICES
        .get(state.live_target_position_index as usize)
        .map(|(_, value)| *value)
        .unwrap_or(0x1)
}

pub fn live_position_index() -> i32 {
    STATE.lock().unwrap().live_target_position_index
}

pub fn live_character_position_index() -> i32 {
    let state = STATE.lock().unwrap();
    let index = state.live_target_position_index;

    LIVE_POSITION_CHOICES
        .get(index as usize)
        .and_then(|(_, flag)| {
            if flag.count_ones() == 1 {
                let index = flag.trailing_zeros() as i32;
                Some(if index >= 18 { index + 2 } else { index })
            }
            else {
                None
            }
        })
        .unwrap_or(0)
}

pub fn live_part() -> i32 {
    let state = STATE.lock().unwrap();
    LIVE_PART_CHOICES
        .get(state.live_target_part_index as usize)
        .map(|(_, value)| *value)
        .unwrap_or(0)
}

pub fn race_model_index() -> i32 {
    let index = STATE.lock().unwrap().race_target_index;
    if index < 0 { 0 } else { index }
}

pub fn update_live_follow_target(target: Vector3_t) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return;
    }

    let mut state = STATE.lock().unwrap();
    let target = Vec3::from(target);
    state.live_follow_precise_target = true;
    state.live_follow_timeline_updated = true;
    update_live_follow_camera_locked(&mut state, &config.free_camera, target);
}

pub fn update_live_follow_position_target(target: Vector3_t) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return;
    }

    let mut state = STATE.lock().unwrap();
    if state.scene != CameraScene::Live || state.mode != FreeCameraMode::SelfieStick {
        return;
    }

    let position_target = Vec3::from(target);
    state.live_follow_position_target = Some(position_target);
    state.live_follow_precise_target = true;
    state.live_follow_timeline_updated = true;
    state.live_selfie_last_head_pos = state.live_selfie_head_pos;
    update_live_follow_camera_locked(&mut state, &config.free_camera, position_target);
}

fn update_live_follow_camera_locked(
    state: &mut FreeCameraState,
    config: &FreeCameraConfig,
    position_target: Vec3,
) {
    let mut position_target = apply_live_selfie_dead_zone_locked(state, config, position_target);
    let had_target = state.live_follow_target.is_some();
    if config.live_follow_smooth && had_target {
        let old_pos_target = state.live_follow_target.unwrap();
        position_target =
            old_pos_target.lerp(position_target, config.live_follow_smooth_pos_step.clamp(0.02, 1.0));
    }
    state.live_follow_target = Some(position_target);

    let look_at = position_target + state.live_follow_lookat_offset;
    let angle = state.live_follow_offset.x.to_radians();
    let distance = state.live_follow_offset.z;
    let camera_pos = Vec3::new(
        look_at.x - angle.sin() * distance,
        look_at.y + state.live_follow_offset.y,
        look_at.z - angle.cos() * distance,
    );
    let camera_look_at = if config.live_follow_smooth && had_target {
        state.camera_look_at.lerp(look_at, config.live_follow_smooth_lookat_step.clamp(0.02, 1.0))
    }
    else {
        look_at
    };
    state.camera_pos = camera_pos;
    state.camera_look_at = camera_look_at;
    state.camera_rotation = None;
}

pub fn refresh_paused_live_camera() {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || config.free_camera.selfie_use_head_transform {
        return;
    }

    let mut state = STATE.lock().unwrap();
    if state.scene != CameraScene::Live || state.mode != FreeCameraMode::SelfieStick {
        return;
    }

    let Some(position_target) = state.live_follow_target else {
        return;
    };
    let look_at = position_target + state.live_follow_lookat_offset;
    let angle = state.live_follow_offset.x.to_radians();
    let distance = state.live_follow_offset.z;
    state.camera_pos = Vec3::new(
        look_at.x - angle.sin() * distance,
        look_at.y + state.live_follow_offset.y,
        look_at.z - angle.cos() * distance,
    );
    state.camera_look_at = look_at;
    state.camera_rotation = None;
}

fn apply_live_selfie_dead_zone_locked(
    state: &mut FreeCameraState,
    config: &FreeCameraConfig,
    position_target: Vec3,
) -> Vec3 {
    let horizontal_dead_zone = config.live_selfie_horizontal_stabilization.max(0.0);
    let vertical_dead_zone = config.live_selfie_vertical_stabilization.max(0.0);
    if config.selfie_use_head_transform ||
        (horizontal_dead_zone <= f32::EPSILON && vertical_dead_zone <= f32::EPSILON) ||
        has_selfie_manual_input(state)
    {
        state.live_selfie_stabilized_target = Some(position_target);
        return position_target;
    }

    if let Some(stabilized_target) = state.live_selfie_stabilized_target {
        let delta = position_target - stabilized_target;
        let horizontal_len = (delta.x * delta.x + delta.z * delta.z).sqrt();
        let vertical_len = delta.y.abs();
        let mut next_target = stabilized_target;

        if horizontal_dead_zone <= f32::EPSILON {
            next_target.x = position_target.x;
            next_target.z = position_target.z;
        }
        else if horizontal_len > horizontal_dead_zone {
            let amount = (horizontal_len - horizontal_dead_zone) / horizontal_len;
            next_target.x += delta.x * amount;
            next_target.z += delta.z * amount;
        }

        if vertical_dead_zone <= f32::EPSILON {
            next_target.y = position_target.y;
        }
        else if vertical_len > vertical_dead_zone {
            next_target.y += delta.y.signum() * (vertical_len - vertical_dead_zone);
        }

        if (next_target - stabilized_target).len() <= f32::EPSILON {
            return stabilized_target;
        }

        state.live_selfie_stabilized_target = Some(next_target);
        return next_target;
    }

    state.live_selfie_stabilized_target = Some(position_target);
    position_target
}

fn has_selfie_manual_input(state: &FreeCameraState) -> bool {
    state.key_state.forward ||
        state.key_state.back ||
        state.key_state.left ||
        state.key_state.right ||
        state.key_state.down ||
        state.key_state.up ||
        state.key_state.look_up ||
        state.key_state.look_down ||
        state.key_state.look_left ||
        state.key_state.look_right ||
        state.key_state.follow_offset_up ||
        state.key_state.follow_offset_down ||
        state.key_state.follow_offset_left ||
        state.key_state.follow_offset_right ||
        state.gamepad.axes.left_x.abs() > 0.01 ||
        state.gamepad.axes.left_y.abs() > 0.01 ||
        state.gamepad.axes.right_x.abs() > 0.01 ||
        state.gamepad.axes.right_y.abs() > 0.01 ||
        state.gamepad.axes.left_trigger.abs() > 0.01 ||
        state.gamepad.axes.right_trigger.abs() > 0.01
}

pub fn update_live_head_part_target(target: Vector3_t) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || !config.free_camera.selfie_use_head_transform {
        return;
    }

    let mut state = STATE.lock().unwrap();
    if state.scene == CameraScene::Live && state.mode == FreeCameraMode::SelfieStick {
        state.live_head_part_target = Some(Vec3::from(target));
    }
}

fn live_part_anchor_from_head(state: &FreeCameraState, head: Vec3, rot: Quat) -> Vec3 {
    let right = rot.rotate_vec(Vec3::new(1.0, 0.0, 0.0));
    let forward = rot.rotate_vec(Vec3::new(0.0, 0.0, 1.0));
    let part = LIVE_PART_CHOICES
        .get(state.live_target_part_index as usize)
        .map(|(_, value)| *value)
        .unwrap_or(0);
    match part {
        0x1 | 0x7 | 0xd | 0x13 => head + Vec3::new(0.0, -0.72, 0.0),
        0x2 => head + right * -0.36 + Vec3::new(0.0, -0.46, 0.0) + forward * -0.04,
        0x3 => head + right * 0.36 + Vec3::new(0.0, -0.46, 0.0) + forward * -0.04,
        0x4 | 0x8 | 0xc | 0x12 => head + Vec3::new(0.0, -0.34, 0.0),
        0x5 | 0xe => head + Vec3::new(0.0, -1.35, 0.0),
        0x9 => head + right * 0.36 + Vec3::new(0.0, -0.46, 0.0) + forward * -0.04,
        0xa => head + right * -0.36 + Vec3::new(0.0, -0.46, 0.0) + forward * -0.04,
        0xf | 0x10 => head + Vec3::new(0.0, -0.85, 0.0),
        _ => head,
    }
}

pub fn update_live_director_follow_target(
    pos: Vector3_t,
    root_pos: Vector3_t,
    rot: Quaternion_t,
    forward: Option<Vector3_t>,
) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return;
    }
    if config.free_camera.selfie_use_head_transform {
        return;
    }

    let rot = Quat::from_quaternion(rot);
    let forward = forward.map(Vec3::from)
        .filter(|value| value.len() > f32::EPSILON)
        .map(|value| value.normalized())
        .unwrap_or_else(|| rot.rotate_vec(Vec3::new(0.0, 0.0, 1.0)).normalized());
    let mut state = STATE.lock().unwrap();
    state.set_scene(CameraScene::Live, &config.free_camera);
    if state.mode != FreeCameraMode::SelfieStick {
        return;
    }

    let head_pos = Vec3::from(pos);
    state.live_selfie_head_pos = Some(head_pos);
    state.live_selfie_head_forward = Some(forward);
    if state.live_follow_timeline_updated {
        state.live_selfie_last_head_pos = Some(head_pos);
        return;
    }
    let position_target = {
        let part = LIVE_PART_CHOICES
            .get(state.live_target_part_index as usize)
            .map(|(_, value)| *value)
            .unwrap_or(0xf);
        if matches!(part, 0xf | 0x10) {
            Vec3::from(root_pos)
        }
        else {
            live_part_anchor_from_head(&state, head_pos, rot)
        }
    };
    if state.live_follow_precise_target {
        let mut position_target = state.live_follow_position_target.unwrap_or(position_target);
        if let Some(last_head_pos) = state.live_selfie_last_head_pos {
            let delta = head_pos - last_head_pos;
            position_target.x += delta.x;
            position_target.y += delta.y;
            state.live_follow_position_target = Some(position_target);
        }
        state.live_selfie_last_head_pos = Some(head_pos);
        update_live_follow_camera_locked(&mut state, &config.free_camera, position_target);
        return;
    }
    state.live_follow_position_target = Some(position_target);
    state.live_selfie_last_head_pos = Some(head_pos);
    update_live_follow_camera_locked(&mut state, &config.free_camera, position_target);
}

pub fn update_live_head_follow(pos: Vector3_t, rot: Quaternion_t, forward: Option<Vector3_t>) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || !config.free_camera.selfie_use_head_transform {
        return;
    }

    let mut state = STATE.lock().unwrap();
    state.set_scene(CameraScene::Live, &config.free_camera);
    if state.mode != FreeCameraMode::SelfieStick {
        return;
    }

    let rot = Quat::from_quaternion(rot);
    let head_pos = Vec3::from(pos);
    let fallback = live_part_anchor_from_head(&state, head_pos, rot);
    let base = state.live_head_part_target.unwrap_or(fallback);
    let right = rot.rotate_vec(Vec3::new(1.0, 0.0, 0.0));
    let up = rot.rotate_vec(Vec3::new(0.0, 1.0, 0.0));
    let forward = forward.map(Vec3::from)
        .filter(|value| value.len() > f32::EPSILON)
        .map(|value| value.normalized())
        .unwrap_or_else(|| rot.rotate_vec(Vec3::new(0.0, 0.0, 1.0)));
    let offset = state.live_follow_offset;
    let look_offset = state.live_follow_lookat_offset;
    let distance = offset.z.abs().max(0.05);

    state.camera_pos = base + right * offset.x + up * offset.y + forward * distance;
    state.camera_look_at =
        base +
        right * look_offset.x +
        up * look_offset.y +
        forward * look_offset.z;
    state.camera_rotation = None;
}

pub fn update_first_person(
    scene: CameraScene,
    pos: Vector3_t,
    rot: Quaternion_t,
    forward: Option<Vector3_t>,
) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return;
    }

    let mut state = STATE.lock().unwrap();
    state.set_scene(scene, &config.free_camera);
    if state.mode != FreeCameraMode::FirstPerson {
        return;
    }

    let base = Vec3::from(pos);
    let mut rot = Quat::from_quaternion(rot);
    if scene == CameraScene::Race {
        rot = rot
            .rotate_axis(state.race_first_person_lookat_offset.y, Vec3::new(1.0, 0.0, 0.0))
            .rotate_axis(state.race_first_person_lookat_offset.x, Vec3::new(0.0, 1.0, 0.0));
    }

    let offset = if scene == CameraScene::Live {
        state.live_first_person_offset
    }
    else {
        Vec3::default()
    };
    let right = rot.rotate_vec(Vec3::new(1.0, 0.0, 0.0));
    let forward = forward.map(Vec3::from)
        .filter(|value| value.len() > f32::EPSILON)
        .map(|value| value.normalized())
        .unwrap_or_else(|| rot.rotate_vec(Vec3::new(0.0, 0.0, 1.0)));
    state.camera_pos = base + right * offset.x + forward * offset.z;
    if scene == CameraScene::Live {
        state.camera_pos.y += offset.y;
    }
    state.camera_look_at = state.camera_pos + forward * LOOK_RADIUS;
    state.camera_rotation = Some(rot);
}

pub fn update_race_head_follow(pos: Vector3_t, rot: Quaternion_t) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled || !config.free_camera.selfie_use_head_transform {
        return;
    }

    let mut state = STATE.lock().unwrap();
    state.set_scene(CameraScene::Race, &config.free_camera);
    if state.mode != FreeCameraMode::SelfieStick {
        return;
    }

    let base = Vec3::from(pos);
    let rot = Quat::from_quaternion(rot);
    let right = rot.rotate_vec(Vec3::new(1.0, 0.0, 0.0));
    let up = rot.rotate_vec(Vec3::new(0.0, 1.0, 0.0));
    let forward = rot.rotate_vec(Vec3::new(0.0, 0.0, 1.0));
    let offset = state.race_follow_offset;
    let look_offset = state.race_first_person_lookat_offset;
    let distance = (offset.z + state.race_follow_distance).abs().max(0.05);

    state.camera_pos =
        base +
        right * offset.x +
        up * offset.y +
        forward * distance;
    state.camera_look_at =
        base +
        right * look_offset.x +
        up * look_offset.y;
    state.camera_rotation = None;
}

pub fn update_race_target(index: i32, pos: Vector3_t, rot: Quaternion_t) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return;
    }

    let mut state = STATE.lock().unwrap();
    if state.race_target_index >= 0 && state.race_target_index != index {
        return;
    }

    let new_target = Vec3::from(pos) + Vec3::new(0.0, 1.0, 0.0);
    if state.race_target_seen &&
        ((new_target.x - state.race_target.x).abs() > f32::EPSILON ||
         (new_target.z - state.race_target.z).abs() > f32::EPSILON)
    {
        state.race_target_last = state.race_target;
    }
    else if !state.race_target_seen {
        state.race_target_last = new_target;
    }

    state.race_target = new_target;
    state.race_target_rot = Quat::from_quaternion(rot);
    state.race_target_seen = true;

    if state.mode == FreeCameraMode::SelfieStick && !config.free_camera.selfie_use_head_transform {
        update_race_follow_locked(&mut state);
    }
}

pub fn race_camera_pos(current: Vector3_t) -> Vector3_t {
    let config = Hachimi::instance().config.load();
    let mut state = STATE.lock().unwrap();
    if state.mode == FreeCameraMode::SelfieStick && !config.free_camera.selfie_use_head_transform {
        update_race_follow_locked(&mut state);
    }
    else if state.mode == FreeCameraMode::Free {
        let _ = current;
    }
    state.camera_pos.to_vector3()
}

fn update_race_follow_locked(state: &mut FreeCameraState) {
    if !state.race_target_seen {
        return;
    }

    let rot = state.race_target_rot;
    let mut forward = rot.rotate_vec(Vec3::new(0.0, 0.0, 1.0));
    forward.y = 0.0;
    forward = if forward.len() > f32::EPSILON {
        forward.normalized()
    }
    else {
        let mut move_dir = state.race_target - state.race_target_last;
        move_dir.y = 0.0;
        if move_dir.len() > f32::EPSILON {
            move_dir.normalized()
        }
        else {
            Vec3::new(0.0, 0.0, 1.0)
        }
    };
    let right = Vec3::new(forward.z, 0.0, -forward.x).normalized();
    let offset = state.race_follow_offset;
    state.camera_pos =
        state.race_target +
        right * offset.x +
        Vec3::new(0.0, offset.y, 0.0) +
        forward * (offset.z + state.race_follow_distance);
    state.camera_look_at =
        state.race_target +
        right * state.race_first_person_lookat_offset.x +
        Vec3::new(0.0, state.race_first_person_lookat_offset.y, 0.0);
    state.camera_rotation = None;
}

pub fn slerp_quaternion(a: Quaternion_t, b: Quaternion_t, t: f32) -> Quaternion_t {
    Quat::from_quaternion(a)
        .slerp(Quat::from_quaternion(b), t)
        .to_quaternion()
}

#[cfg(target_os = "windows")]
pub fn on_windows_key(vk: u16, pressed: bool, repeat: bool) {
    if !is_game_input_capture_active() {
        return;
    }

    let config = Hachimi::instance().config.load();
    let kb = &config.free_camera.keybinds;
    let mut state = STATE.lock().unwrap();
    set_key_flag(&mut state.key_state, vk, pressed, kb);

    if !pressed || repeat {
        return;
    }

    if vk == kb.reset {
        state.reset_current_mode_camera(&config.free_camera);
    }
    else if vk == kb.cycle_mode {
        cycle_mode_locked(&mut state);
    }
    else if vk == kb.reverse {
        reverse_locked(&mut state);
    }
    else if vk == kb.target_previous {
        previous_target_locked(&mut state);
    }
    else if vk == kb.target_next {
        next_target_locked(&mut state);
    }
    else if vk == kb.part_previous {
        previous_live_part_locked(&mut state);
    }
    else if vk == kb.part_next {
        next_live_part_locked(&mut state);
    }
}

#[cfg(target_os = "windows")]
pub fn is_windows_key_bound(vk: u16) -> bool {
    if !is_game_input_capture_active() {
        return false;
    }

    let config = Hachimi::instance().config.load();
    let kb = &config.free_camera.keybinds;
    vk == kb.move_forward ||
        vk == kb.move_back ||
        vk == kb.move_left ||
        vk == kb.move_right ||
        vk == kb.move_down ||
        vk == kb.move_up ||
        vk == kb.look_up ||
        vk == kb.look_down ||
        vk == kb.look_left ||
        vk == kb.look_right ||
        vk == kb.fov_increase ||
        vk == kb.fov_decrease ||
        vk == kb.follow_offset_up ||
        vk == kb.follow_offset_down ||
        vk == kb.follow_offset_left ||
        vk == kb.follow_offset_right ||
        vk == kb.target_previous ||
        vk == kb.target_next ||
        vk == kb.part_previous ||
        vk == kb.part_next ||
        vk == kb.reset ||
        vk == kb.cycle_mode ||
        vk == kb.reverse
}

#[cfg(target_os = "windows")]
fn set_key_flag(state: &mut KeyState, vk: u16, pressed: bool, kb: &FreeCameraKeybinds) {
    if vk == kb.move_forward { state.forward = pressed; }
    if vk == kb.move_back { state.back = pressed; }
    if vk == kb.move_left { state.left = pressed; }
    if vk == kb.move_right { state.right = pressed; }
    if vk == kb.move_down { state.down = pressed; }
    if vk == kb.move_up { state.up = pressed; }
    if vk == kb.look_up { state.look_up = pressed; }
    if vk == kb.look_down { state.look_down = pressed; }
    if vk == kb.look_left { state.look_left = pressed; }
    if vk == kb.look_right { state.look_right = pressed; }
    if vk == kb.fov_increase { state.fov_increase = pressed; }
    if vk == kb.fov_decrease { state.fov_decrease = pressed; }
    if vk == kb.follow_offset_up { state.follow_offset_up = pressed; }
    if vk == kb.follow_offset_down { state.follow_offset_down = pressed; }
    if vk == kb.follow_offset_left { state.follow_offset_left = pressed; }
    if vk == kb.follow_offset_right { state.follow_offset_right = pressed; }
}

#[cfg(target_os = "windows")]
pub fn wants_windows_input_capture() -> bool {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return false;
    }

    let state = STATE.lock().unwrap();
    state.right_mouse_down ||
        state.key_state.forward ||
        state.key_state.back ||
        state.key_state.left ||
        state.key_state.right ||
        state.key_state.down ||
        state.key_state.up ||
        state.key_state.look_up ||
        state.key_state.look_down ||
        state.key_state.look_left ||
        state.key_state.look_right ||
        state.key_state.fov_increase ||
        state.key_state.fov_decrease ||
        state.key_state.follow_offset_up ||
        state.key_state.follow_offset_down ||
        state.key_state.follow_offset_left ||
        state.key_state.follow_offset_right
}

#[cfg(not(target_os = "windows"))]
pub fn wants_windows_input_capture() -> bool {
    false
}

pub fn on_mouse_button(right_down: bool) {
    if !is_game_input_capture_active() {
        return;
    }

    let mut state = STATE.lock().unwrap();
    state.right_mouse_down = right_down;
    state.last_mouse_pos = None;
}

pub fn on_mouse_move(x: i32, y: i32) {
    let config = Hachimi::instance().config.load();
    if !config.free_camera.enabled {
        return;
    }

    let mut state = STATE.lock().unwrap();
    if !state.right_mouse_down {
        state.last_mouse_pos = Some((x, y));
        return;
    }

    let Some((last_x, last_y)) = state.last_mouse_pos else {
        state.last_mouse_pos = Some((x, y));
        return;
    };
    state.last_mouse_pos = Some((x, y));
    let dx = (x - last_x) as f32;
    let dy = (y - last_y) as f32;
    let speed = config.free_camera.mouse_speed / 100.0;
    apply_look_delta_locked(&mut state, -dx * speed, -dy * speed, true);
}

pub fn on_mouse_wheel(delta: i16) {
    if !is_game_input_capture_active() {
        return;
    }

    let mut state = STATE.lock().unwrap();
    let step = if delta > 0 { 0.5 } else { -0.5 };
    change_fov_locked(&mut state, step);
}

pub fn on_gamepad_axes(axes: GamepadAxes) {
    if !is_enabled() {
        return;
    }

    STATE.lock().unwrap().gamepad.axes = axes;
}

pub fn on_gamepad_button(button: GamepadButton, pressed: bool) {
    if !is_enabled() {
        return;
    }

    let mut state = STATE.lock().unwrap();
    match button {
        GamepadButton::LeftBumper => state.gamepad.lb = pressed,
        GamepadButton::RightBumper => state.gamepad.rb = pressed,
        _ if pressed => match button {
            GamepadButton::A => request_toggle_live_pause_locked(&state),
            GamepadButton::B => reverse_locked(&mut state),
            GamepadButton::X => cycle_mode_locked(&mut state),
            GamepadButton::Y => {
                let config = Hachimi::instance().config.load();
                state.reset_current_mode_camera(&config.free_camera);
            },
            GamepadButton::DpadLeft => previous_target_locked(&mut state),
            GamepadButton::DpadRight => next_target_locked(&mut state),
            GamepadButton::DpadUp => next_live_part_locked(&mut state),
            GamepadButton::DpadDown => previous_live_part_locked(&mut state),
            _ => (),
        },
        _ => (),
    }
}

#[derive(Clone, Copy, Debug)]
pub enum GamepadButton {
    A,
    B,
    X,
    Y,
    LeftBumper,
    RightBumper,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

pub fn tick() {
    let config = Hachimi::instance().config.load();
    let config = &config.free_camera;
    let mut state = STATE.lock().unwrap();

    if RELOAD_CONFIG_REQUESTED.swap(false, Ordering::AcqRel) ||
        (config.enabled && !state.last_enabled) ||
        config.mode != state.last_config_mode
    {
        state.reset_with_config(config);
    }
    state.last_enabled = config.enabled;
    if !config.enabled {
        return;
    }
    if !matches!(state.scene, CameraScene::Live | CameraScene::Race) {
        state.last_tick = Instant::now();
        return;
    }
    if state.scene == CameraScene::Race && state.mode != state.last_overlay_mode {
        state.last_overlay_mode = state.mode;
        set_overlay_message(t!(
            "free_camera.overlay_mode",
            mode = mode_label(state.mode)
        ).into_owned());
    }

    #[cfg(target_os = "windows")]
    poll_unity_gamepad_locked(&mut state, config);

    let now = Instant::now();
    let delta = now.duration_since(state.last_tick).as_secs_f32();
    state.last_tick = now;
    let step_scale = (delta / 0.01).clamp(0.25, 4.0);
    let move_step = match state.scene {
        CameraScene::Race => config.race_move_step,
        _ => config.live_move_step,
    } * step_scale;
    let look_step = config.look_step * step_scale;

    apply_input_locked(&mut state, config, move_step, look_step);
}

fn apply_input_locked(
    state: &mut FreeCameraState,
    config: &FreeCameraConfig,
    move_step: f32,
    look_step: f32,
) {
    let mut forward = bool_axis(state.key_state.forward, state.key_state.back);
    let mut side = bool_axis(state.key_state.left, state.key_state.right);
    let mut vertical = bool_axis(state.key_state.up, state.key_state.down);
    let mut look_x = bool_axis(state.key_state.look_left, state.key_state.look_right);
    let mut look_y = bool_axis(state.key_state.look_up, state.key_state.look_down);

    let axes = state.gamepad.axes;
    forward += deadzone(axes.left_y, config.gamepad_deadzone) * config.gamepad_move_speed;
    side -= deadzone(axes.left_x, config.gamepad_deadzone) * config.gamepad_move_speed;
    vertical += (axes.right_trigger - axes.left_trigger) * config.gamepad_move_speed;
    look_x -= deadzone(axes.right_x, config.gamepad_deadzone) * config.gamepad_look_speed;
    look_y += deadzone(axes.right_y, config.gamepad_deadzone) * config.gamepad_look_speed;

    if state.gamepad.lb {
        change_fov_locked(state, 0.5 * move_step.max(0.1));
    }
    if state.gamepad.rb {
        change_fov_locked(state, -0.5 * move_step.max(0.1));
    }
    if state.key_state.fov_increase {
        change_fov_locked(state, 0.5 * move_step.max(0.1));
    }
    if state.key_state.fov_decrease {
        change_fov_locked(state, -0.5 * move_step.max(0.1));
    }

    if state.key_state.follow_offset_up {
        adjust_follow_offset_y_locked(state, move_step / 3.0);
    }
    if state.key_state.follow_offset_down {
        adjust_follow_offset_y_locked(state, -move_step / 3.0);
    }
    if state.key_state.follow_offset_left {
        adjust_follow_offset_x_locked(state, move_step * 10.0);
    }
    if state.key_state.follow_offset_right {
        adjust_follow_offset_x_locked(state, -move_step * 10.0);
    }

    if forward.abs() > f32::EPSILON {
        move_forward_locked(state, forward * move_step);
    }
    if side.abs() > f32::EPSILON {
        move_side_locked(state, side * move_step);
    }
    if vertical.abs() > f32::EPSILON {
        move_vertical_locked(state, vertical * move_step);
    }
    if look_x.abs() > f32::EPSILON || look_y.abs() > f32::EPSILON {
        apply_look_delta_locked(state, look_x * look_step, look_y * look_step, false);
    }
}

fn bool_axis(positive: bool, negative: bool) -> f32 {
    positive as i32 as f32 - negative as i32 as f32
}

fn deadzone(value: f32, deadzone: f32) -> f32 {
    if value.abs() < deadzone {
        0.0
    }
    else {
        value
    }
}

fn move_forward_locked(state: &mut FreeCameraState, amount: f32) {
    match state.mode {
        FreeCameraMode::Free => {
            let yaw = state.yaw.to_radians();
            let pitch = state.pitch.to_radians();
            let dir = Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                -yaw.cos() * pitch.cos(),
            );
            state.camera_pos = state.camera_pos + dir * amount;
            state.camera_look_at = state.camera_look_at + dir * amount;
        },
        FreeCameraMode::SelfieStick => {
            let head_selfie = Hachimi::instance().config.load().free_camera.selfie_use_head_transform;
            if state.scene == CameraScene::Live {
                state.live_follow_offset.z -= amount / 2.0;
            }
            else if head_selfie {
                state.race_follow_offset.z -= amount / 2.0;
            }
            else {
                state.race_follow_offset.z += amount / 2.0;
                state.race_follow_distance += amount / 2.0;
            }
        },
        FreeCameraMode::FirstPerson => {
            if state.scene == CameraScene::Live {
                state.live_first_person_offset.z =
                    (state.live_first_person_offset.z + amount * 0.025).clamp(-1.0, 1.0);
            }
        },
    }
}

fn move_side_locked(state: &mut FreeCameraState, amount: f32) {
    match state.mode {
        FreeCameraMode::Free => {
            let yaw = state.yaw.to_radians();
            let dir = Vec3::new(yaw.cos(), 0.0, yaw.sin());
            state.camera_pos = state.camera_pos + dir * amount;
            state.camera_look_at = state.camera_look_at + dir * amount;
        },
        FreeCameraMode::SelfieStick => {
            let head_selfie = Hachimi::instance().config.load().free_camera.selfie_use_head_transform;
            if state.scene == CameraScene::Live && !head_selfie {
                state.live_follow_lookat_offset.x += amount;
            }
            else {
                adjust_follow_offset_x_locked(state, amount);
            }
        },
        FreeCameraMode::FirstPerson => (),
    }
}

fn move_vertical_locked(state: &mut FreeCameraState, amount: f32) {
    match state.mode {
        FreeCameraMode::Free => {
            state.camera_pos.y += amount;
            state.camera_look_at.y += amount;
        },
        FreeCameraMode::SelfieStick => {
            let head_selfie = Hachimi::instance().config.load().free_camera.selfie_use_head_transform;
            if state.scene == CameraScene::Live && !head_selfie {
                state.live_follow_lookat_offset.y += amount / 2.0;
            }
            else {
                adjust_follow_offset_y_locked(state, amount / 2.0);
            }
        },
        FreeCameraMode::FirstPerson => {
            if state.scene == CameraScene::Live {
                state.live_first_person_offset.y =
                    (state.live_first_person_offset.y + amount * 0.025).clamp(-1.0, 1.0);
            }
        },
    }
}

fn apply_look_delta_locked(state: &mut FreeCameraState, yaw_delta: f32, pitch_delta: f32, mouse: bool) {
    match state.mode {
        FreeCameraMode::Free => {
            state.yaw += yaw_delta;
            if state.yaw >= 360.0 {
                state.yaw -= 720.0;
            }
            if state.yaw <= -360.0 {
                state.yaw += 720.0;
            }
            state.pitch = (state.pitch + pitch_delta).clamp(-89.99, 89.99);
            state.update_look_from_angles();
        },
        FreeCameraMode::SelfieStick => {
            if state.scene == CameraScene::Live {
                state.live_follow_offset.x += yaw_delta * 2.0;
                state.live_follow_offset.y += pitch_delta;
            }
            else {
                state.race_first_person_lookat_offset.x -= yaw_delta;
                state.race_first_person_lookat_offset.y += if mouse { pitch_delta / 2.0 } else { pitch_delta };
            }
        },
        FreeCameraMode::FirstPerson => {
            if state.scene == CameraScene::Race {
                state.race_first_person_lookat_offset.x -= yaw_delta;
                state.race_first_person_lookat_offset.y += pitch_delta;
            }
        },
    }
}

fn adjust_follow_offset_x_locked(state: &mut FreeCameraState, value: f32) {
    if state.scene == CameraScene::Live && state.mode == FreeCameraMode::SelfieStick {
        state.live_follow_offset.x += value * 2.0;
    }
    else if state.scene == CameraScene::Race && state.mode == FreeCameraMode::SelfieStick {
        state.race_first_person_lookat_offset.x -= value;
        state.race_follow_offset.x += value / 4.0;
    }
}

fn adjust_follow_offset_y_locked(state: &mut FreeCameraState, value: f32) {
    if state.scene == CameraScene::Live && state.mode == FreeCameraMode::SelfieStick {
        state.live_follow_offset.y += value;
    }
    else if state.scene == CameraScene::Race && state.mode == FreeCameraMode::SelfieStick {
        state.race_follow_offset.y += value / 2.0;
    }
}

fn change_fov_locked(state: &mut FreeCameraState, value: f32) {
    match state.scene {
        CameraScene::Live => state.live_fov = (state.live_fov + value).clamp(1.0, 120.0),
        CameraScene::Race => state.race_fov = (state.race_fov + value).clamp(1.0, 120.0),
        CameraScene::None => (),
    }
}

fn cycle_mode_locked(state: &mut FreeCameraState) {
    state.mode = match state.mode {
        FreeCameraMode::Free => FreeCameraMode::SelfieStick,
        FreeCameraMode::SelfieStick => FreeCameraMode::FirstPerson,
        FreeCameraMode::FirstPerson => FreeCameraMode::Free,
    };
    state.camera_rotation = None;
    state.live_follow_target = None;
    state.live_head_part_target = None;
    state.live_follow_precise_target = false;
    state.live_follow_timeline_updated = false;
    state.live_selfie_camera_offset = None;
    state.live_selfie_look_offset = None;
    state.live_selfie_last_head_pos = None;
    state.live_selfie_stabilized_target = None;
    state.last_overlay_mode = state.mode;
    set_overlay_message(t!(
        "free_camera.overlay_mode",
        mode = mode_label(state.mode)
    ).into_owned());
}

fn reverse_locked(state: &mut FreeCameraState) {
    if state.scene == CameraScene::Race {
        state.race_follow_offset.z = -state.race_follow_offset.z;
        state.race_follow_distance = -state.race_follow_distance;
    }
    else if state.scene == CameraScene::Live && state.mode == FreeCameraMode::SelfieStick {
        state.live_follow_offset.z = -state.live_follow_offset.z;
    }
}

fn previous_target_locked(state: &mut FreeCameraState) {
    let old_live_index = state.live_target_position_index;
    let old_race_index = state.race_target_index;

    if state.scene == CameraScene::Race {
        state.race_target_index -= 1;
        if state.race_target_index < -1 {
            state.race_target_index = -1;
        }
        if state.race_target_index != old_race_index {
            set_overlay_message(t!(
                "free_camera.overlay_target",
                target = race_target_label(state.race_target_index)
            ).into_owned());
        }
    }
    else if state.scene == CameraScene::Live &&
        (state.mode == FreeCameraMode::SelfieStick || state.mode == FreeCameraMode::FirstPerson)
    {
        state.live_target_position_index =
            (state.live_target_position_index - 1).rem_euclid(LIVE_POSITION_CHOICES.len() as i32);
        if state.live_target_position_index != old_live_index {
            state.live_follow_target = None;
            state.live_head_part_target = None;
            state.live_follow_precise_target = false;
            state.live_follow_timeline_updated = false;
            state.live_selfie_camera_offset = None;
            state.live_selfie_look_offset = None;
            state.live_selfie_last_head_pos = None;
            state.live_selfie_stabilized_target = None;
            set_overlay_message(t!(
                "free_camera.overlay_target",
                target = live_target_label(state.live_target_position_index)
            ).into_owned());
        }
    }
}

fn next_target_locked(state: &mut FreeCameraState) {
    let old_live_index = state.live_target_position_index;
    let old_race_index = state.race_target_index;

    if state.scene == CameraScene::Race {
        state.race_target_index += 1;
        if state.race_target_index > 17 {
            state.race_target_index = -1;
        }
        if state.race_target_index != old_race_index {
            set_overlay_message(t!(
                "free_camera.overlay_target",
                target = race_target_label(state.race_target_index)
            ).into_owned());
        }
    }
    else if state.scene == CameraScene::Live &&
        (state.mode == FreeCameraMode::SelfieStick || state.mode == FreeCameraMode::FirstPerson)
    {
        state.live_target_position_index =
            (state.live_target_position_index + 1).rem_euclid(LIVE_POSITION_CHOICES.len() as i32);
        if state.live_target_position_index != old_live_index {
            state.live_follow_target = None;
            state.live_head_part_target = None;
            state.live_follow_precise_target = false;
            state.live_follow_timeline_updated = false;
            state.live_selfie_camera_offset = None;
            state.live_selfie_look_offset = None;
            state.live_selfie_last_head_pos = None;
            state.live_selfie_stabilized_target = None;
            set_overlay_message(t!(
                "free_camera.overlay_target",
                target = live_target_label(state.live_target_position_index)
            ).into_owned());
        }
    }
}

fn previous_live_part_locked(state: &mut FreeCameraState) {
    if state.scene != CameraScene::Race && state.mode == FreeCameraMode::SelfieStick {
        let old_index = state.live_target_part_index;
        state.live_target_part_index =
            (state.live_target_part_index - 1).rem_euclid(LIVE_PART_CHOICES.len() as i32);
        if state.live_target_part_index != old_index {
            state.live_follow_target = None;
            state.live_head_part_target = None;
            state.live_follow_precise_target = false;
            state.live_follow_timeline_updated = false;
            state.live_selfie_stabilized_target = None;
            set_overlay_message(t!(
                "free_camera.overlay_part",
                part = live_part_label(state.live_target_part_index)
            ).into_owned());
        }
    }
}

fn next_live_part_locked(state: &mut FreeCameraState) {
    if state.scene != CameraScene::Race && state.mode == FreeCameraMode::SelfieStick {
        let old_index = state.live_target_part_index;
        state.live_target_part_index =
            (state.live_target_part_index + 1).rem_euclid(LIVE_PART_CHOICES.len() as i32);
        if state.live_target_part_index != old_index {
            state.live_follow_target = None;
            state.live_head_part_target = None;
            state.live_follow_precise_target = false;
            state.live_follow_timeline_updated = false;
            state.live_selfie_stabilized_target = None;
            set_overlay_message(t!(
                "free_camera.overlay_part",
                part = live_part_label(state.live_target_part_index)
            ).into_owned());
        }
    }
}

#[cfg(target_os = "windows")]
fn poll_unity_gamepad_locked(state: &mut FreeCameraState, config: &FreeCameraConfig) {
    use crate::il2cpp::hook::Unity_InputSystem as input;

    let Some(gamepad) = input::current_gamepad_state() else {
        state.gamepad.axes = GamepadAxes::default();
        state.gamepad.lb = false;
        state.gamepad.rb = false;
        state.gamepad.last_buttons = 0;
        return;
    };

    state.gamepad.axes = GamepadAxes {
        left_x: gamepad.left_x,
        left_y: gamepad.left_y,
        right_x: gamepad.right_x,
        right_y: gamepad.right_y,
        left_trigger: gamepad.left_trigger,
        right_trigger: gamepad.right_trigger,
    };
    state.gamepad.lb = gamepad.buttons & input::LEFT_SHOULDER != 0;
    state.gamepad.rb = gamepad.buttons & input::RIGHT_SHOULDER != 0;

    let pressed = gamepad.buttons & !state.gamepad.last_buttons;
    state.gamepad.last_buttons = gamepad.buttons;

    for (mask, button) in [
        (input::BUTTON_SOUTH, GamepadButton::A),
        (input::BUTTON_EAST, GamepadButton::B),
        (input::BUTTON_WEST, GamepadButton::X),
        (input::BUTTON_NORTH, GamepadButton::Y),
        (input::DPAD_UP, GamepadButton::DpadUp),
        (input::DPAD_DOWN, GamepadButton::DpadDown),
        (input::DPAD_LEFT, GamepadButton::DpadLeft),
        (input::DPAD_RIGHT, GamepadButton::DpadRight),
    ] {
        if pressed & mask != 0 {
            match button {
                GamepadButton::A => request_toggle_live_pause_locked(state),
                GamepadButton::B => reverse_locked(state),
                GamepadButton::X => cycle_mode_locked(state),
                GamepadButton::Y => state.reset_current_mode_camera(config),
                GamepadButton::DpadLeft => previous_target_locked(state),
                GamepadButton::DpadRight => next_target_locked(state),
                GamepadButton::DpadUp => next_live_part_locked(state),
                GamepadButton::DpadDown => previous_live_part_locked(state),
                _ => (),
            }
        }
    }
}

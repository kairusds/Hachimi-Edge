use std::{ffi::c_void, ptr::null_mut};

use crate::il2cpp::{
    symbols::{get_assembly_image, get_class, get_method_addr},
    types::*,
};

static mut GET_CURRENT_GAMEPAD_ADDR: usize = 0;
static mut GET_LEFT_STICK_ADDR: usize = 0;
static mut GET_RIGHT_STICK_ADDR: usize = 0;
static mut GET_DPAD_ADDR: usize = 0;
static mut GET_LEFT_TRIGGER_ADDR: usize = 0;
static mut GET_RIGHT_TRIGGER_ADDR: usize = 0;
static mut GET_LEFT_SHOULDER_ADDR: usize = 0;
static mut GET_RIGHT_SHOULDER_ADDR: usize = 0;
static mut GET_BUTTON_SOUTH_ADDR: usize = 0;
static mut GET_BUTTON_EAST_ADDR: usize = 0;
static mut GET_BUTTON_WEST_ADDR: usize = 0;
static mut GET_BUTTON_NORTH_ADDR: usize = 0;
static mut GET_START_BUTTON_ADDR: usize = 0;
static mut GET_VECTOR2_X_ADDR: usize = 0;
static mut GET_VECTOR2_Y_ADDR: usize = 0;
static mut GET_DPAD_UP_ADDR: usize = 0;
static mut GET_DPAD_DOWN_ADDR: usize = 0;
static mut GET_DPAD_LEFT_ADDR: usize = 0;
static mut GET_DPAD_RIGHT_ADDR: usize = 0;
static mut GET_CURRENT_STATE_PTR_ADDR: usize = 0;
static mut READ_AXIS_UNPROCESSED_ADDR: usize = 0;
static mut GET_BUTTON_IS_PRESSED_ADDR: usize = 0;

pub const DPAD_UP: u16 = 0x0001;
pub const DPAD_DOWN: u16 = 0x0002;
pub const DPAD_LEFT: u16 = 0x0004;
pub const DPAD_RIGHT: u16 = 0x0008;
pub const START: u16 = 0x0010;
pub const LEFT_SHOULDER: u16 = 0x0100;
pub const RIGHT_SHOULDER: u16 = 0x0200;
pub const BUTTON_SOUTH: u16 = 0x1000;
pub const BUTTON_EAST: u16 = 0x2000;
pub const BUTTON_WEST: u16 = 0x4000;
pub const BUTTON_NORTH: u16 = 0x8000;

#[derive(Clone, Copy, Debug, Default)]
pub struct GamepadSnapshot {
    pub buttons: u16,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
}

fn get_control(gamepad: *mut Il2CppObject, addr: usize) -> *mut Il2CppObject {
    if gamepad.is_null() || addr == 0 {
        return null_mut();
    }
    let getter: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(addr) };
    getter(gamepad)
}

fn read_axis(control: *mut Il2CppObject) -> f32 {
    let get_state_addr = unsafe { GET_CURRENT_STATE_PTR_ADDR };
    let read_addr = unsafe { READ_AXIS_UNPROCESSED_ADDR };
    if control.is_null() || get_state_addr == 0 || read_addr == 0 {
        return 0.0;
    }
    let get_state: extern "C" fn(*mut Il2CppObject) -> *mut c_void =
        unsafe { std::mem::transmute(get_state_addr) };
    let state = get_state(control);
    if state.is_null() {
        return 0.0;
    }
    let read: extern "C" fn(*mut Il2CppObject, *mut c_void) -> f32 =
        unsafe { std::mem::transmute(read_addr) };
    read(control, state)
}

fn read_stick(control: *mut Il2CppObject) -> Vector2_t {
    if control.is_null() {
        return Vector2_t::default();
    }
    Vector2_t {
        x: read_axis(get_control(control, unsafe { GET_VECTOR2_X_ADDR })),
        y: read_axis(get_control(control, unsafe { GET_VECTOR2_Y_ADDR })),
    }
}

fn is_pressed(control: *mut Il2CppObject) -> bool {
    let addr = unsafe { GET_BUTTON_IS_PRESSED_ADDR };
    if control.is_null() || addr == 0 {
        return false;
    }
    let get: extern "C" fn(*mut Il2CppObject) -> bool = unsafe { std::mem::transmute(addr) };
    get(control)
}

pub fn current_gamepad_state() -> Option<GamepadSnapshot> {
    let get_current_addr = unsafe { GET_CURRENT_GAMEPAD_ADDR };
    if get_current_addr == 0 {
        return None;
    }
    let get_current: extern "C" fn() -> *mut Il2CppObject =
        unsafe { std::mem::transmute(get_current_addr) };
    let gamepad = get_current();
    if gamepad.is_null() {
        return None;
    }

    let left = read_stick(get_control(gamepad, unsafe { GET_LEFT_STICK_ADDR }));
    let right = read_stick(get_control(gamepad, unsafe { GET_RIGHT_STICK_ADDR }));
    let dpad = get_control(gamepad, unsafe { GET_DPAD_ADDR });
    let mut buttons = 0;
    if is_pressed(get_control(dpad, unsafe { GET_DPAD_UP_ADDR })) {
        buttons |= DPAD_UP;
    }
    if is_pressed(get_control(dpad, unsafe { GET_DPAD_DOWN_ADDR })) {
        buttons |= DPAD_DOWN;
    }
    if is_pressed(get_control(dpad, unsafe { GET_DPAD_LEFT_ADDR })) {
        buttons |= DPAD_LEFT;
    }
    if is_pressed(get_control(dpad, unsafe { GET_DPAD_RIGHT_ADDR })) {
        buttons |= DPAD_RIGHT;
    }
    if is_pressed(get_control(gamepad, unsafe { GET_START_BUTTON_ADDR })) {
        buttons |= START;
    }
    if is_pressed(get_control(gamepad, unsafe { GET_LEFT_SHOULDER_ADDR })) {
        buttons |= LEFT_SHOULDER;
    }
    if is_pressed(get_control(gamepad, unsafe { GET_RIGHT_SHOULDER_ADDR })) {
        buttons |= RIGHT_SHOULDER;
    }
    if is_pressed(get_control(gamepad, unsafe { GET_BUTTON_SOUTH_ADDR })) {
        buttons |= BUTTON_SOUTH;
    }
    if is_pressed(get_control(gamepad, unsafe { GET_BUTTON_EAST_ADDR })) {
        buttons |= BUTTON_EAST;
    }
    if is_pressed(get_control(gamepad, unsafe { GET_BUTTON_WEST_ADDR })) {
        buttons |= BUTTON_WEST;
    }
    if is_pressed(get_control(gamepad, unsafe { GET_BUTTON_NORTH_ADDR })) {
        buttons |= BUTTON_NORTH;
    }

    Some(GamepadSnapshot {
        buttons,
        left_trigger: read_axis(get_control(gamepad, unsafe { GET_LEFT_TRIGGER_ADDR })),
        right_trigger: read_axis(get_control(gamepad, unsafe { GET_RIGHT_TRIGGER_ADDR })),
        left_x: left.x,
        left_y: left.y,
        right_x: right.x,
        right_y: right.y,
    })
}

pub fn init() {
    let image = match get_assembly_image(c"Unity.InputSystem.dll") {
        Ok(image) => image,
        Err(e) => {
            warn!("Unity Input System unavailable: {}", e);
            return;
        }
    };
    let gamepad = match get_class(image, c"UnityEngine.InputSystem", c"Gamepad") {
        Ok(class) => class,
        Err(e) => {
            warn!("Unity Gamepad class unavailable: {}", e);
            return;
        }
    };
    let axis_control = match get_class(image, c"UnityEngine.InputSystem.Controls", c"AxisControl") {
        Ok(class) => class,
        Err(e) => {
            warn!("Unity AxisControl class unavailable: {}", e);
            return;
        }
    };
    let vector2_control = match get_class(
        image,
        c"UnityEngine.InputSystem.Controls",
        c"Vector2Control",
    ) {
        Ok(class) => class,
        Err(e) => {
            warn!("Unity Vector2Control class unavailable: {}", e);
            return;
        }
    };
    let input_control = match get_class(image, c"UnityEngine.InputSystem", c"InputControl") {
        Ok(class) => class,
        Err(e) => {
            warn!("Unity InputControl class unavailable: {}", e);
            return;
        }
    };
    let dpad_control = match get_class(image, c"UnityEngine.InputSystem.Controls", c"DpadControl") {
        Ok(class) => class,
        Err(e) => {
            warn!("Unity DpadControl class unavailable: {}", e);
            return;
        }
    };
    let button_control =
        match get_class(image, c"UnityEngine.InputSystem.Controls", c"ButtonControl") {
            Ok(class) => class,
            Err(e) => {
                warn!("Unity ButtonControl class unavailable: {}", e);
                return;
            }
        };

    unsafe {
        GET_CURRENT_GAMEPAD_ADDR = get_method_addr(gamepad, c"get_current", 0);
        GET_LEFT_STICK_ADDR = get_method_addr(gamepad, c"get_leftStick", 0);
        GET_RIGHT_STICK_ADDR = get_method_addr(gamepad, c"get_rightStick", 0);
        GET_DPAD_ADDR = get_method_addr(gamepad, c"get_dpad", 0);
        GET_LEFT_TRIGGER_ADDR = get_method_addr(gamepad, c"get_leftTrigger", 0);
        GET_RIGHT_TRIGGER_ADDR = get_method_addr(gamepad, c"get_rightTrigger", 0);
        GET_LEFT_SHOULDER_ADDR = get_method_addr(gamepad, c"get_leftShoulder", 0);
        GET_RIGHT_SHOULDER_ADDR = get_method_addr(gamepad, c"get_rightShoulder", 0);
        GET_BUTTON_SOUTH_ADDR = get_method_addr(gamepad, c"get_buttonSouth", 0);
        GET_BUTTON_EAST_ADDR = get_method_addr(gamepad, c"get_buttonEast", 0);
        GET_BUTTON_WEST_ADDR = get_method_addr(gamepad, c"get_buttonWest", 0);
        GET_BUTTON_NORTH_ADDR = get_method_addr(gamepad, c"get_buttonNorth", 0);
        GET_START_BUTTON_ADDR = get_method_addr(gamepad, c"get_startButton", 0);
        GET_VECTOR2_X_ADDR = get_method_addr(vector2_control, c"get_x", 0);
        GET_VECTOR2_Y_ADDR = get_method_addr(vector2_control, c"get_y", 0);
        GET_DPAD_UP_ADDR = get_method_addr(dpad_control, c"get_up", 0);
        GET_DPAD_DOWN_ADDR = get_method_addr(dpad_control, c"get_down", 0);
        GET_DPAD_LEFT_ADDR = get_method_addr(dpad_control, c"get_left", 0);
        GET_DPAD_RIGHT_ADDR = get_method_addr(dpad_control, c"get_right", 0);
        GET_CURRENT_STATE_PTR_ADDR = get_method_addr(input_control, c"get_currentStatePtr", 0);
        READ_AXIS_UNPROCESSED_ADDR =
            get_method_addr(axis_control, c"ReadUnprocessedValueFromState", 1);
        GET_BUTTON_IS_PRESSED_ADDR = get_method_addr(button_control, c"get_isPressed", 0);
    }
}

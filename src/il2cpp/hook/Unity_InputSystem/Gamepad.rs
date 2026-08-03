use crate::il2cpp::{symbols::get_method_addr, types::*};

use super::{AxisControl, ButtonControl, DpadControl, Vector2Control};

static mut GET_CURRENT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_current, GET_CURRENT_ADDR, *mut Il2CppObject,);

static mut GET_LEFT_STICK_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_leftStick, GET_LEFT_STICK_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_RIGHT_STICK_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_rightStick, GET_RIGHT_STICK_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_DPAD_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_dpad, GET_DPAD_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_LEFT_TRIGGER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_leftTrigger, GET_LEFT_TRIGGER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_RIGHT_TRIGGER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_rightTrigger, GET_RIGHT_TRIGGER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_LEFT_SHOULDER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_leftShoulder, GET_LEFT_SHOULDER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_RIGHT_SHOULDER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_rightShoulder, GET_RIGHT_SHOULDER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_BUTTON_SOUTH_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_buttonSouth, GET_BUTTON_SOUTH_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_BUTTON_EAST_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_buttonEast, GET_BUTTON_EAST_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_BUTTON_WEST_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_buttonWest, GET_BUTTON_WEST_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_BUTTON_NORTH_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_buttonNorth, GET_BUTTON_NORTH_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_START_BUTTON_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_startButton, GET_START_BUTTON_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

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
pub struct GamepadAxes {
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum GamepadButton {
    A,
    B,
    X,
    Y,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

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

fn read_stick(control: *mut Il2CppObject) -> Vector2_t {
    if control.is_null() {
        return Vector2_t::default();
    }
    Vector2_t {
        x: AxisControl::read_unprocessed_value(Vector2Control::get_x(control)),
        y: AxisControl::read_unprocessed_value(Vector2Control::get_y(control)),
    }
}

pub fn current_gamepad_state() -> Option<GamepadSnapshot> {
    let gamepad = get_current();
    if gamepad.is_null() {
        return None;
    }

    let left = read_stick(get_leftStick(gamepad));
    let right = read_stick(get_rightStick(gamepad));
    let dpad = get_dpad(gamepad);
    let mut buttons = 0;
    if ButtonControl::get_isPressed(DpadControl::get_up(dpad)) {
        buttons |= DPAD_UP;
    }
    if ButtonControl::get_isPressed(DpadControl::get_down(dpad)) {
        buttons |= DPAD_DOWN;
    }
    if ButtonControl::get_isPressed(DpadControl::get_left(dpad)) {
        buttons |= DPAD_LEFT;
    }
    if ButtonControl::get_isPressed(DpadControl::get_right(dpad)) {
        buttons |= DPAD_RIGHT;
    }
    if ButtonControl::get_isPressed(get_startButton(gamepad)) {
        buttons |= START;
    }
    if ButtonControl::get_isPressed(get_leftShoulder(gamepad)) {
        buttons |= LEFT_SHOULDER;
    }
    if ButtonControl::get_isPressed(get_rightShoulder(gamepad)) {
        buttons |= RIGHT_SHOULDER;
    }
    if ButtonControl::get_isPressed(get_buttonSouth(gamepad)) {
        buttons |= BUTTON_SOUTH;
    }
    if ButtonControl::get_isPressed(get_buttonEast(gamepad)) {
        buttons |= BUTTON_EAST;
    }
    if ButtonControl::get_isPressed(get_buttonWest(gamepad)) {
        buttons |= BUTTON_WEST;
    }
    if ButtonControl::get_isPressed(get_buttonNorth(gamepad)) {
        buttons |= BUTTON_NORTH;
    }

    Some(GamepadSnapshot {
        buttons,
        left_trigger: AxisControl::read_unprocessed_value(get_leftTrigger(gamepad)),
        right_trigger: AxisControl::read_unprocessed_value(get_rightTrigger(gamepad)),
        left_x: left.x,
        left_y: left.y,
        right_x: right.x,
        right_y: right.y,
    })
}

pub fn init(Unity_InputSystem: *const Il2CppImage) {
    get_class_or_return!(Unity_InputSystem, "UnityEngine.InputSystem", Gamepad);

    unsafe {
        GET_CURRENT_ADDR = get_method_addr(Gamepad, c"get_current", 0);
        GET_LEFT_STICK_ADDR = get_method_addr(Gamepad, c"get_leftStick", 0);
        GET_RIGHT_STICK_ADDR = get_method_addr(Gamepad, c"get_rightStick", 0);
        GET_DPAD_ADDR = get_method_addr(Gamepad, c"get_dpad", 0);
        GET_LEFT_TRIGGER_ADDR = get_method_addr(Gamepad, c"get_leftTrigger", 0);
        GET_RIGHT_TRIGGER_ADDR = get_method_addr(Gamepad, c"get_rightTrigger", 0);
        GET_LEFT_SHOULDER_ADDR = get_method_addr(Gamepad, c"get_leftShoulder", 0);
        GET_RIGHT_SHOULDER_ADDR = get_method_addr(Gamepad, c"get_rightShoulder", 0);
        GET_BUTTON_SOUTH_ADDR = get_method_addr(Gamepad, c"get_buttonSouth", 0);
        GET_BUTTON_EAST_ADDR = get_method_addr(Gamepad, c"get_buttonEast", 0);
        GET_BUTTON_WEST_ADDR = get_method_addr(Gamepad, c"get_buttonWest", 0);
        GET_BUTTON_NORTH_ADDR = get_method_addr(Gamepad, c"get_buttonNorth", 0);
        GET_START_BUTTON_ADDR = get_method_addr(Gamepad, c"get_startButton", 0);
    }
}

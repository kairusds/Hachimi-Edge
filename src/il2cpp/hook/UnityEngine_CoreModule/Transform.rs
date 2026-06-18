use crate::{
    il2cpp::{
        api::{il2cpp_class_get_type, il2cpp_resolve_icall, il2cpp_type_get_object},
        symbols::get_method_addr,
        types::*
    }
};

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

// public Transform get_parent() { }
static mut GET_PARENT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_parent, GET_PARENT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

// public Int32 get_childCount() { }
static mut GET_CHILDCOUNT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_childCount, GET_CHILDCOUNT_ADDR, i32, this: *mut Il2CppObject);

// public Transform GetChild(Int32 index) { }
static mut GETCHILD_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetChild, GETCHILD_ADDR, *mut Il2CppObject, this: *mut Il2CppObject, index: i32);

static mut GET_POSITION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_position_Injected, GET_POSITION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Vector3_t);

static mut SET_POSITION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_position_Injected, SET_POSITION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Vector3_t);

static mut GET_LOCALPOSITION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_localPosition_Injected, GET_LOCALPOSITION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Vector3_t);

static mut SET_LOCALPOSITION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_localPosition_Injected, SET_LOCALPOSITION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Vector3_t);

static mut GET_ROTATION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_rotation_Injected, GET_ROTATION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Quaternion_t);

static mut GET_FORWARD_ADDR: usize = 0;
pub fn get_forward(ret: *mut Vector3_t, this: *mut Il2CppObject) -> *mut Vector3_t {
    if unsafe { GET_FORWARD_ADDR } == 0 {
        return ret;
    }
    let orig_fn: extern "C" fn(*mut Vector3_t, *mut Il2CppObject) -> *mut Vector3_t =
        unsafe { std::mem::transmute(GET_FORWARD_ADDR) };
    orig_fn(ret, this)
}

static mut SET_ROTATION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_rotation_Injected, SET_ROTATION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Quaternion_t);

static mut GET_LOCALROTATION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_localRotation_Injected, GET_LOCALROTATION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Quaternion_t);

static mut SET_LOCALROTATION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_localRotation_Injected, SET_LOCALROTATION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Quaternion_t);

static mut INTERNAL_LOOKAT_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(
    Internal_LookAt_Injected,
    INTERNAL_LOOKAT_INJECTED_ADDR,
    (),
    this: *mut Il2CppObject,
    world_position: *mut Vector3_t,
    world_up: *mut Vector3_t
);

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Transform);

    unsafe {
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(Transform));
        GET_PARENT_ADDR = get_method_addr(Transform, c"get_parent", 0);
        GET_CHILDCOUNT_ADDR = get_method_addr(Transform, c"get_childCount", 0);
        GETCHILD_ADDR = get_method_addr(Transform, c"GetChild", 1);
        GET_POSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::get_position_Injected(UnityEngine.Vector3&)".as_ptr());
        SET_POSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::set_position_Injected(UnityEngine.Vector3&)".as_ptr());
        GET_LOCALPOSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::get_localPosition_Injected(UnityEngine.Vector3&)".as_ptr());
        SET_LOCALPOSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::set_localPosition_Injected(UnityEngine.Vector3&)".as_ptr());
        GET_ROTATION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::get_rotation_Injected(UnityEngine.Quaternion&)".as_ptr());
        GET_FORWARD_ADDR = get_method_addr(Transform, c"get_forward", 0);
        SET_ROTATION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::set_rotation_Injected(UnityEngine.Quaternion&)".as_ptr());
        GET_LOCALROTATION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::get_localRotation_Injected(UnityEngine.Quaternion&)".as_ptr());
        SET_LOCALROTATION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::set_localRotation_Injected(UnityEngine.Quaternion&)".as_ptr());
        INTERNAL_LOOKAT_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::Internal_LookAt_Injected(UnityEngine.Vector3&,UnityEngine.Vector3&)".as_ptr());
    }
}

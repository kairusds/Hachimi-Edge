use std::ptr::null_mut;
use crate::{il2cpp::{ext::StringExt, symbols::{get_method_addr, SingletonLike}, types::*}};
use super::{DialogCommon::{Data, FormType}, TextId};

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return null_mut();
    };
    singleton.instance()
}

static mut PUSHDIALOG_ADDR: usize = 0;
impl_addr_wrapper_fn!(PushDialog, PUSHDIALOG_ADDR, (), data: *mut Il2CppObject);

// public static DialogCommon PushSystemDialog(Data dialogData, Boolean isEnableOutsideClick) { }
static mut PUSHSYSTEMDIALOG_ADDR: usize = 0;
impl_addr_wrapper_fn!(PushSystemDialog, PUSHSYSTEMDIALOG_ADDR, (), dialogData: *mut Il2CppObject, isEnableOutsideClick: bool);

pub fn single_button_message(title: &str, message: &str, typ: FormType) {
    let dialog_data = Data::new();
    Data::SetSimpleOneButtonMessage(
        dialog_data,
        title.to_il2cpp_string(),
        message.to_il2cpp_string(),
        null_mut(),
        TextId::from_name("Common0007"),
        typ
    );
    PushDialog(dialog_data);
    // PushSystemDialog(dialog_data, true);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DialogManager);

    unsafe {
        CLASS = DialogManager;
        PUSHDIALOG_ADDR = get_method_addr(DialogManager, c"PushDialog", 1);
        PUSHSYSTEMDIALOG_ADDR =get_method_addr(DialogManager, c"PushSystemDialog", 2);
    }
}

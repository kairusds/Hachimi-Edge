use crate::{
    core::{template, Hachimi},
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        hook::UnityEngine_TextRenderingModule::TextAnchor,
        symbols::{get_field_from_name, get_method_addr},
        types::*,
    },
};

def_field_value_accessors!(get_lineSpace, set_lineSpace, LINESPACE_FIELD, f32);
def_field_value_accessors!(get_fontSize, set_fontSize, FONTSIZE_FIELD, i32);
def_field_value_accessors!(get_textAnchor, set_textAnchor, TEXTANCHOR_FIELD, i32);
def_field_value_accessors!(get_useFit, set_useFit, USEFIT_FIELD, bool);
def_field_value_accessors!(get_useWrap, set_useWrap, USEWRAP_FIELD, bool);

static mut SET_TEXT_FIT_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextFit, SET_TEXT_FIT_ADDR, (), this: *mut Il2CppObject, enable: bool);

static mut SET_TEXT_WRAP_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextWrap, SET_TEXT_WRAP_ADDR, (), this: *mut Il2CppObject, enable: bool);

static mut SET_TEXT_ANCHOR_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextAnchor, SET_TEXT_ANCHOR_ADDR, (), this: *mut Il2CppObject, anchor: TextAnchor);

static mut SET_TEXT_LINESPACE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextLinespace, SET_TEXT_LINESPACE_ADDR, (), this: *mut Il2CppObject, lineSpace: f32);

static mut SET_TEXT_FONTSIZE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextFontSize, SET_TEXT_FONTSIZE_ADDR, (), this: *mut Il2CppObject, fontSize: i32);


type SetTextFn = extern "C" fn(this: *mut Il2CppObject, text: *mut Il2CppString);
extern "C" fn SetText(this: *mut Il2CppObject, mut text: *mut Il2CppString) {
    let text_utf = unsafe { (*text).as_utf16str() };
    if !text_utf.as_slice().contains(&36) { // 36 = dollar sign ($)
        return get_orig_fn!(SetText, SetTextFn)(this, text);
    }

    // Rationale: AnText has fields and functions. The functions set the fields + update display.
    // Setting fields alone does not update current display, but does use them next time.
    // We store state, possibly modify current through templates, and restore state for next use.

    let line_space = get_lineSpace(this);
    let anchor = get_textAnchor(this);
    let font_size = get_fontSize(this);
    let fit = get_useFit(this);

    text = Hachimi::instance()
        .template_parser
        .eval_with_context(&text_utf.to_string(), &mut TemplateContext { component: this })
        .to_il2cpp_string();
    get_orig_fn!(SetText, SetTextFn)(this, text);

    set_lineSpace(this, line_space);
    set_textAnchor(this, anchor);
    set_fontSize(this, font_size);
    set_useFit(this, fit);
}

struct TemplateContext {
    component: *mut Il2CppObject,
}

impl template::Context for TemplateContext {
    fn on_filter_eval(&mut self, name: &str, args: &[template::Token]) -> Option<String> {
        debug!("Eval filter");
        match name {
            "anchor" => {
                let value = args.get(0)?;
                let template::Token::NumberLit(anchor_num) = *value else {
                    return None;
                };
                if let Ok(anchor) = TextAnchor::try_from(anchor_num as i32 - 1) {
                    SetTextAnchor(self.component, anchor);
                }
            }

            "scale" => {
                debug!("Eval scale");
                let value = args.get(0)?;
                let template::Token::NumberLit(percentage) = value else {
                    return None;
                };
                let cur_size = get_fontSize(self.component);
                debug!("Cur size: {cur_size}");
                let new_size = (cur_size as f64 * (percentage / 100.0)) as i32;
                debug!("Setting scale");
                SetTextFontSize(self.component, new_size);
                debug!("Setting fit");
                SetTextFit(self.component, false);
                debug!("all set");
            }

            "ls" => {
                let value = args.get(0)?;
                let template::Token::NumberLit(ls) = *value else {
                    return None;
                };
                SetTextLinespace(self.component, ls as f32);
            }

            "afit" => {
                let value = args.get(0)?;
                let template::Token::NumberLit(state) = *value else {
                    return None;
                };
                SetTextFit(self.component, state != 0.0);
            }

            "wrap" => {
                let state = args.get(0)?;
                let template::Token::NumberLit(state) = *state else {
                    return None;
                };
                SetTextWrap(self.component, state != 0.0);
            }

            _ => return None,
        }

        Some(String::new())
    }
}

// Context that ignores AnText filters
pub struct IgnoreATFiltersContext();

impl template::Context for IgnoreATFiltersContext {
    fn on_filter_eval(&mut self, _name: &str, _args: &[template::Token]) -> Option<String> {
        match _name {
            "anchor" | "scale" | "ls" | "afit" | "wrap" => Some(String::new()),
            _ => None
        }
    }
}

pub fn init(Plugins: *const Il2CppImage) {
    get_class_or_return!(Plugins, AnimateToUnity, AnText);

    let SetText_addr = get_method_addr(AnText, c"SetText", 1);
    new_hook!(SetText_addr, SetText);

    unsafe {
        USEFIT_FIELD = get_field_from_name(AnText, c"_useFit");
        USEWRAP_FIELD = get_field_from_name(AnText, c"_useWrap");
        TEXTANCHOR_FIELD = get_field_from_name(AnText, c"_textAnchor");
        LINESPACE_FIELD = get_field_from_name(AnText, c"_lineSpace");
        FONTSIZE_FIELD = get_field_from_name(AnText, c"_fontSize");

        SET_TEXT_FIT_ADDR = get_method_addr(AnText, c"SetTextFit", 1);
        SET_TEXT_WRAP_ADDR = get_method_addr(AnText, c"SetTextWrap", 1);
        SET_TEXT_ANCHOR_ADDR = get_method_addr(AnText, c"SetTextAnchor", 1);
        SET_TEXT_LINESPACE_ADDR = get_method_addr(AnText, c"SetTextLinespace", 1);
        SET_TEXT_FONTSIZE_ADDR = get_method_addr(AnText, c"SetTextFontSize", 1);
    }
}

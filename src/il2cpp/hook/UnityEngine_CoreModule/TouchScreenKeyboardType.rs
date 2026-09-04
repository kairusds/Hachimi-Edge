#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub enum TouchScreenKeyboardType {
    Default,
    ASCIICapable,
    NumbersAndPunctuation,
    URL,
    NumberPad,
    PhonePad,
    NamePhonePad,
    EmailAddress,
    NintendoNetworkAccount,
    Social,
    Search,
    DecimalPad,
    OneTimeCode
}

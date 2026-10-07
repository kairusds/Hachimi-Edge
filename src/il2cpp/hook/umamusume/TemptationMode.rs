#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum TemptationMode {
    Null = 0,
    PositionSashi = 1,
    PositionSenko = 2,
    PositionNige = 3,
    Boost = 4
}

impl_enum_eq!(TemptationMode);

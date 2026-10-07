#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Motivation {
    None = 0,
    Min = 1,
    Low = 2,
    Middle = 3,
    High = 4,
    Max = 5
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum LaneType {
    Uti = 0,
    Naka = 1,
    Soto = 2,
    Oosoto = 3
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum HorsePhase {
    RunUp = -1,
    Start = 0,
    MiddleRun = 1,
    End = 2,
    Last = 3,
    Finished = 4
}

impl_enum_eq!(Motivation);
impl_enum_eq!(LaneType);
impl_enum_eq!(HorsePhase);

use crate::il2cpp::types::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialOrd, PartialEq)]
#[repr(i32)]
pub enum BgSeason {
    #[default] None = 0,
    Spring = 1,
    Summer = 2,
    Fall = 3,
    Winter = 4,
    CherryBlossom = 5
}

impl_enum_eq!(BgSeason);
impl_enum_ord!(BgSeason);

pub fn init(_UnityEngine_CoreModule: *const Il2CppImage) {
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SimulateEventType {
    Score = 0,
    ChallengeMatchPoint = 1,
    NOUSE_2 = 2,
    Skill = 3,
    CompeteTop = 4,
    CompeteFight = 5,
    ReleaseConservePower = 6,
    StaminaLimitBreakBuff = 7,
    CompeteBeforeSpurt = 8,
    StaminaKeep = 9,
    SecureLead = 10,
    Performance = 11,
    RunAtFullSpeed = 12,
    LastSpurt = 13,
    Temptation = 14,
    BadStart = 15
}

impl_enum_eq!(SimulateEventType);
impl_enum_ord!(SimulateEventType);

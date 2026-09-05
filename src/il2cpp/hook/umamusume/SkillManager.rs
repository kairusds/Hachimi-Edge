use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
};

static mut GET_USED_SKILL_ID_LIST_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetUsedSkillIdList, GET_USED_SKILL_ID_LIST_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_SKILLS_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetSkills, GET_SKILLS_ADDR, *mut Il2CppArray, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, SkillManager);

    unsafe {
        GET_USED_SKILL_ID_LIST_ADDR = get_method_addr(SkillManager, c"GetUsedSkillIdList", 0);
        GET_SKILLS_ADDR = get_method_addr(SkillManager, c"GetSkills", 0);
    }
}

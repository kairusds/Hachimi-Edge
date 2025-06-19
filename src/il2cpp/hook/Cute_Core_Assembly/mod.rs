use crate::core::{game::Region, Hachimi};

mod SafetyNet;
mod Device;

pub fn init() {
    get_assembly_image_or_return!(image, "Cute.Core.Assembly.dll");

    match Hachimi::instance().game.region {
        // These versions don't have SafetyNet implemented
        Region::Taiwan | Region::China => (),

        _ => SafetyNet::init(image)
    }
    Device::init(image);
}
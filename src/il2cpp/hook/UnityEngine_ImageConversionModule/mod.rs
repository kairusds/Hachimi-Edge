use crate::core::{game::Region, Hachimi};

pub mod ImageConversion;

pub fn init() {
    if Hachimi::instance().game.region == Region::China {
        return;
    }

    get_assembly_image_or_return!(image, "UnityEngine.ImageConversionModule.dll");

    ImageConversion::init(image);
}
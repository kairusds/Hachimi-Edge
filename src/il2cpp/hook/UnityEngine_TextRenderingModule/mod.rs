pub mod TextGenerator;
pub mod Font;

#[repr(i32)]
pub enum TextAnchor {
    UpperLeft,
    UpperCenter,
    UpperRight,
    MiddleLeft,
    MiddleCenter,
    MiddleRight,
    LowerLeft,
    LowerCenter,
    LowerRight
}
impl TryFrom<i32> for TextAnchor {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error > {
        if value < 0 || value > 8 {
            return Err(());
        }
        Ok(unsafe { std::mem::transmute(value) })
    }
}

pub fn init() {
    get_assembly_image_or_return!(image, "UnityEngine.TextRenderingModule.dll");

    TextGenerator::init(image);
    Font::init(image);
}

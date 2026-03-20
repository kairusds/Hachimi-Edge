pub mod Text;
pub mod CanvasScaler;
pub mod EventSystem;
pub mod LayoutElement;
pub mod LayoutRebuilder;
pub mod Image;
pub mod LayoutGroup;
pub mod VerticalLayoutGroup;
pub mod HorizontalOrVerticalLayoutGroup;
pub mod ContentSizeFitter;

pub fn init() {
    get_assembly_image_or_return!(image, "UnityEngine.UI.dll");
    
    Text::init(image);
    CanvasScaler::init(image);
    EventSystem::init(image);
    LayoutElement::init(image);
    LayoutRebuilder::init(image);
    Image::init(image);
    LayoutGroup::init(image);
    HorizontalOrVerticalLayoutGroup::init(image);
    ContentSizeFitter::init(image);
}
mod MovieManager;
pub mod AtomSourceEx;
pub mod AudioControllerBase;
pub mod CuteAudioSource;
pub mod CuteAudioSourcePool;

pub fn init() {
    get_assembly_image_or_return!(image, "Cute.Cri.Assembly.dll");

    MovieManager::init(image);
    AudioControllerBase::init(image);
    AtomSourceEx::init(image);
    CuteAudioSource::init(image);
    CuteAudioSourcePool::init(image);
}
pub mod CriAtomExAcb;
pub mod CriAtomExPlayer;
pub mod CriAtomSourceBase;

pub fn init() {
    get_assembly_image_or_return!(image, "CriMw.CriWare.Runtime.dll");

    CriAtomExAcb::init(image);
    CriAtomExPlayer::init(image);
    CriAtomSourceBase::init(image);
}

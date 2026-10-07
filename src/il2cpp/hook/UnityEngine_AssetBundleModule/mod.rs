pub mod AssetBundle;
mod AssetBundleRequest;
pub mod AssetBundleCreateRequest;
pub mod AsyncOperation;

pub fn init() {
    get_assembly_image_or_return!(image, "UnityEngine.AssetBundleModule.dll");

    AssetBundle::init(image);
    AssetBundleRequest::init(image);
    AssetBundleCreateRequest::init(image);
    AsyncOperation::init(image);
}

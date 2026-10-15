mod ScriptableRenderer;
pub mod ScreenSpaceAmbientOcclusion;
pub mod UniversalRenderPipelineAsset;

pub use UniversalRenderPipelineAsset::SoftShadowQuality;

pub fn init() {
    get_assembly_image_or_return!(image, "Unity.RenderPipelines.Universal.Runtime.dll");

    ScriptableRenderer::init(image);
    ScreenSpaceAmbientOcclusion::init(image);
    UniversalRenderPipelineAsset::init(image);
}

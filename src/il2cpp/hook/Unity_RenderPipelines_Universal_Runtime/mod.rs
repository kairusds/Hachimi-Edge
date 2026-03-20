mod ScriptableRenderer;

pub fn init() {
    get_assembly_image_or_return!(image, "Unity.RenderPipelines.Universal.Runtime.dll");

    ScriptableRenderer::init(image);
}

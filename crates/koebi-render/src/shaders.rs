//! Embedded SPIR-V, compiled from `shaders/*.vert|frag` by `cargo xtask shaders`.
//!
//! CI recompiles the GLSL with the pinned Vulkan SDK and fails if these binaries are stale.

pub(crate) static RECT_VERT: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/rect.vert.spv"));
pub(crate) static RECT_FRAG: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/rect.frag.spv"));
pub(crate) static LINE_VERT: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/line.vert.spv"));
pub(crate) static LINE_FRAG: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/line.frag.spv"));
pub(crate) static GLYPH_VERT: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/glyph.vert.spv"));
pub(crate) static GLYPH_FRAG: &[u32] =
    koebi_gpu::include_spirv!(include_bytes!("../shaders/spirv/glyph.frag.spv"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_modules_are_spirv() {
        for module in [
            RECT_VERT, RECT_FRAG, LINE_VERT, LINE_FRAG, GLYPH_VERT, GLYPH_FRAG,
        ] {
            assert_eq!(module.first(), Some(&koebi_gpu::SPIRV_MAGIC));
        }
    }
}

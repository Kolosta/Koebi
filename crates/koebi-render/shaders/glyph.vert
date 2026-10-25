#version 450
// Instanced glyph quads, sampled from the glyph atlas.
//
// One instance per glyph, drawn as a 4-vertex triangle strip over the glyph's bitmap. The
// renderer already placed the bitmap on whole physical pixels, so each fragment reads exactly
// one atlas texel. Keep the instance layout in sync with `GlyphInstance`
// (crates/koebi-render/src/atlas.rs) and the push constants with `ViewportConstants`
// (crates/koebi-render/src/renderer.rs).

layout(push_constant) uniform ViewportConstants {
    vec2 viewport_px;   // render target size, physical pixels
    float scale_factor; // unused: glyph quads are already in physical pixels
    float _padding;
} pc;

layout(location = 0) in vec4 in_rect;  // x, y, width, height (physical px)
layout(location = 1) in vec4 in_uv;    // x, y, width, height (normalized atlas coordinates)
layout(location = 2) in vec4 in_color; // premultiplied, gamma-encoded

layout(location = 0) out vec2 v_uv;
layout(location = 1) flat out vec4 v_color;

void main() {
    // Strip order (0,0) (1,0) (0,1) (1,1) covers the quad with two triangles.
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));
    vec2 position = in_rect.xy + corner * in_rect.zw;
    // Vulkan clip space: y points down, so the top-left pixel maps to (-1, -1).
    gl_Position = vec4(position / pc.viewport_px * 2.0 - 1.0, 0.0, 1.0);
    v_uv = in_uv.xy + corner * in_uv.zw;
    v_color = in_color;
}

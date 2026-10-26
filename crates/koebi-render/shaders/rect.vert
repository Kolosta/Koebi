#version 450
// Instanced signed-distance-field rectangle.
//
// One instance per rectangle, drawn as a 4-vertex triangle strip. The quad is inflated by
// AA_MARGIN physical pixels so the fragment shader can compute anti-aliased coverage.
// Keep the instance layout in sync with `RectInstance` (crates/koebi-render/src/scene.rs)
// and the push constants with `ViewportConstants` (crates/koebi-render/src/renderer.rs).

layout(push_constant) uniform ViewportConstants {
    vec2 viewport_px;   // render target size, physical pixels
    float scale_factor; // logical -> physical pixels
    float _padding;
} pc;

layout(location = 0) in vec4 in_rect;          // x, y, width, height (logical px)
layout(location = 1) in vec4 in_fill;          // premultiplied, gamma-encoded
layout(location = 2) in vec4 in_border_color;  // premultiplied, gamma-encoded
layout(location = 3) in vec4 in_radii;         // top-left, top-right, bottom-right, bottom-left
layout(location = 4) in vec4 in_borders;       // top, right, bottom, left (logical px)

layout(location = 0) out vec2 v_local;                // position relative to the center (px)
layout(location = 1) flat out vec2 v_half_size;       // px
layout(location = 2) flat out vec4 v_fill;
layout(location = 3) flat out vec4 v_border_color;
layout(location = 4) flat out vec4 v_radii;           // px, each at most half the smallest side
layout(location = 5) flat out vec4 v_borders;         // px

const float AA_MARGIN = 1.0;

void main() {
    // Strip order (0,0) (1,0) (0,1) (1,1) covers the quad with two triangles.
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));

    vec2 origin = in_rect.xy * pc.scale_factor;
    vec2 half_size = 0.5 * in_rect.zw * pc.scale_factor;
    vec2 center = origin + half_size;
    vec2 local = (corner * 2.0 - 1.0) * (half_size + AA_MARGIN);

    // Vulkan clip space: y points down, so the top-left pixel maps to (-1, -1).
    vec2 ndc = (center + local) / pc.viewport_px * 2.0 - 1.0;
    gl_Position = vec4(ndc, 0.0, 1.0);

    v_local = local;
    v_half_size = half_size;
    v_fill = in_fill;
    v_border_color = in_border_color;
    v_radii = min(in_radii * pc.scale_factor, vec4(min(half_size.x, half_size.y)));
    v_borders = in_borders * pc.scale_factor;
}

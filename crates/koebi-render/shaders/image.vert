#version 450
#extension GL_GOOGLE_include_directive : require
// Instanced image: a texture shown in a rectangle with a radius per corner.
//
// One instance per image, drawn as a 4-vertex triangle strip, its edges on the pixel grid like
// rectangles' unless it asks for exact ones: the part of the texture it shows spans the
// rectangle as drawn, so that one texel per physical pixel shows each texel exactly. Keep the
// instance layout in sync with `ImageInstance` (crates/koebi-render/src/image.rs) and the push
// constants with `ViewportConstants` (crates/koebi-render/src/renderer.rs).

layout(push_constant) uniform ViewportConstants {
    vec2 viewport_px;   // render target size, physical pixels
    float scale_factor; // logical -> physical pixels
    float linear;       // 1 when the image's texels are linear light
} pc;

#include "clip_vertex.glsl"

layout(location = 0) in vec4 in_rect;   // x, y, width, height (logical px)
layout(location = 1) in vec4 in_uv;     // the part shown: u0, v0, u1, v1
layout(location = 2) in vec4 in_radii;  // top-left, top-right, bottom-right, bottom-left
layout(location = 3) in vec4 in_params; // x: opacity, y: exact edges

layout(location = 0) out vec2 v_local;          // position relative to the center (px)
layout(location = 1) out vec2 v_uv;             // texture coordinates
layout(location = 2) flat out vec2 v_half_size; // px
layout(location = 3) flat out vec4 v_radii;     // px
layout(location = 4) flat out float v_opacity;

const float AA_MARGIN = 1.0;
// How far below a half pixel a position still rounds up (TIE_TOLERANCE in scene.rs).
const float TIE = 1.0 / 256.0;

void main() {
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));
    vec2 low = in_rect.xy * pc.scale_factor;
    vec2 high = (in_rect.xy + in_rect.zw) * pc.scale_factor;
    if (in_params.y < 0.5) {
        // Each edge to the nearest pixel edge, as rectangles do.
        low = floor(low + 0.5 + TIE);
        high = floor(high + 0.5 + TIE);
    }
    vec2 half_size = 0.5 * (high - low);
    vec2 center = low + half_size;
    vec2 local = (corner * 2.0 - 1.0) * (half_size + AA_MARGIN);

    vec2 ndc = (center + local) / pc.viewport_px * 2.0 - 1.0;
    // An image snapped to no width or no height covers nothing.
    if (half_size.x <= 0.0 || half_size.y <= 0.0) {
        ndc = vec2(-2.0);
    }
    gl_Position = vec4(ndc, 0.0, 1.0);

    // Across the rectangle as drawn; the margin past it reads the edge texels.
    vec2 along = (local + half_size) / max(2.0 * half_size, vec2(1e-6));
    v_uv = mix(in_uv.xy, in_uv.zw, along);
    v_local = local;
    v_half_size = half_size;
    v_radii = min(max(in_radii, vec4(0.0)) * pc.scale_factor, vec4(min(half_size.x, half_size.y)));
    v_opacity = clamp(in_params.x, 0.0, 1.0);
    pass_clip();
}

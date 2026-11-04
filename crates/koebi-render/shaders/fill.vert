#version 450
// Instanced fill: a shape whose color the fragment shader computes for each pixel.
//
// One instance per fill, drawn as a 4-vertex triangle strip, its edges on the pixel grid like
// rectangles' unless it asks for exact ones. Keep the instance layout in sync with
// `FillInstance` (crates/koebi-render/src/fill.rs) and the push constants with
// `ViewportConstants` (crates/koebi-render/src/renderer.rs).

layout(push_constant) uniform ViewportConstants {
    vec2 viewport_px;   // render target size, physical pixels
    float scale_factor; // logical -> physical pixels
    float _padding;
} pc;

layout(location = 0) in vec4 in_rect;    // x, y, width, height (logical px)
layout(location = 1) in vec4 in_radii;   // top-left, top-right, bottom-right, bottom-left
layout(location = 2) in vec4 in_color0;  // gamma-encoded, straight alpha
layout(location = 3) in vec4 in_color1;
layout(location = 4) in vec4 in_color2;
layout(location = 5) in vec4 in_color3;
layout(location = 6) in vec4 in_params;  // kind, then its parameters
layout(location = 7) in vec4 in_options; // x: exact edges, y: a fourth parameter

layout(location = 0) out vec2 v_local;          // position relative to the center (px)
layout(location = 1) flat out vec2 v_half_size; // px
layout(location = 2) flat out vec4 v_radii;     // px
layout(location = 3) flat out vec4 v_color0;
layout(location = 4) flat out vec4 v_color1;
layout(location = 5) flat out vec4 v_color2;
layout(location = 6) flat out vec4 v_color3;
layout(location = 7) flat out vec4 v_params;
layout(location = 8) flat out float v_extra;

const float AA_MARGIN = 1.0;
// How far below a half pixel a position still rounds up (TIE_TOLERANCE in scene.rs).
const float TIE = 1.0 / 256.0;

void main() {
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));
    vec2 low = in_rect.xy * pc.scale_factor;
    vec2 high = (in_rect.xy + in_rect.zw) * pc.scale_factor;
    if (in_options.x < 0.5) {
        // Each edge to the nearest pixel edge, as rectangles do: fills meet them exactly.
        low = floor(low + 0.5 + TIE);
        high = floor(high + 0.5 + TIE);
    }
    vec2 half_size = 0.5 * (high - low);
    vec2 center = low + half_size;
    vec2 local = (corner * 2.0 - 1.0) * (half_size + AA_MARGIN);

    vec2 ndc = (center + local) / pc.viewport_px * 2.0 - 1.0;
    // A fill snapped to no width or no height covers nothing: its quad collapses, so that no
    // fragment divides by its size.
    if (half_size.x <= 0.0 || half_size.y <= 0.0) {
        ndc = vec2(-2.0);
    }
    gl_Position = vec4(ndc, 0.0, 1.0);

    v_local = local;
    v_half_size = half_size;
    v_radii = min(in_radii * pc.scale_factor, vec4(min(half_size.x, half_size.y)));
    v_color0 = in_color0;
    v_color1 = in_color1;
    v_color2 = in_color2;
    v_color3 = in_color3;
    v_params = in_params;
    // A checkerboard's square side and an arrow's outline, in physical pixels.
    if (in_params.x > 0.5 && in_params.x < 1.5) {
        v_params.y = in_params.y * pc.scale_factor;
    }
    if (in_params.x > 6.5) {
        v_params.z = in_params.z * pc.scale_factor;
    }
    v_extra = in_options.y;
}

#version 450
#extension GL_GOOGLE_include_directive : require
// Instanced vector icon: a square whose pixels the fragment shader covers from the icon's curves.
//
// One instance per icon, drawn as a 4-vertex triangle strip over the largest square centered in
// its rectangle, on whole physical pixels: its center goes to the nearest half pixel and its side
// to the nearest whole number of pixels of the parity that keeps both edges on pixel edges (odd
// on a pixel's center, even on an edge). An icon drawn on the pixel grid keeps its edges on it,
// and one centered on a pixel's center or on a line keeps its axis there. Then turned about its
// center by the rotation.
// Keep the instance layout in sync with `IconInstance` (crates/koebi-render/src/icon.rs) and the
// push constants with `ViewportConstants` (crates/koebi-render/src/renderer.rs).

layout(push_constant) uniform ViewportConstants {
    vec2 viewport_px;   // render target size, physical pixels
    float scale_factor; // logical -> physical pixels
    float _padding;
} pc;

#include "clip_vertex.glsl"

layout(location = 0) in vec4 in_rect;   // x, y, width, height (logical px)
layout(location = 1) in vec4 in_params; // icon, mask of the zones recolored, opacity, rotation
layout(location = 2) in vec4 in_color0; // zone colors, gamma-encoded, straight alpha
layout(location = 3) in vec4 in_color1;
layout(location = 4) in vec4 in_color2;
layout(location = 5) in vec4 in_color3;
layout(location = 6) in vec4 in_color4;
layout(location = 7) in vec4 in_color5;
layout(location = 8) in vec4 in_color6;
layout(location = 9) in vec4 in_color7;

// The square's center (physical px) and its turn's cosine and sine: the fragment shader finds
// each pixel's point in the unit square from these and gl_FragCoord, nothing interpolated.
layout(location = 0) flat out vec4 v_place;
layout(location = 1) flat out vec2 v_side;   // the square's width and height, physical px
layout(location = 2) flat out vec4 v_params;
layout(location = 3) flat out vec4 v_color0;
layout(location = 4) flat out vec4 v_color1;
layout(location = 5) flat out vec4 v_color2;
layout(location = 6) flat out vec4 v_color3;
layout(location = 7) flat out vec4 v_color4;
layout(location = 8) flat out vec4 v_color5;
layout(location = 9) flat out vec4 v_color6;
layout(location = 10) flat out vec4 v_color7;

// How far below a half pixel a position still rounds up (TIE_TOLERANCE in scene.rs).
const float TIE = 1.0 / 256.0;

void main() {
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));
    float side = min(in_rect.z, in_rect.w);
    vec2 origin = in_rect.xy + 0.5 * (in_rect.zw - vec2(side));
    vec2 center = (origin + 0.5 * side) * pc.scale_factor;
    float wanted = side * pc.scale_factor;
    // The center in half pixels: even on a pixel edge, odd on a pixel's center.
    vec2 halves = floor(center * 2.0 + 0.5 + TIE);
    vec2 side_px = vec2(floor(wanted + 0.5 + TIE));
    // A side of the other parity moves by a pixel toward the size asked for.
    vec2 mismatch = mod(side_px + halves, 2.0);
    side_px += mismatch * vec2(wanted >= side_px.x ? 1.0 : -1.0, wanted >= side_px.y ? 1.0 : -1.0);
    vec2 low = 0.5 * (halves - side_px);

    // Clockwise on screen (y down), about the square's center.
    vec2 local = (corner - 0.5) * side_px;
    float c = cos(in_params.w);
    float s = sin(in_params.w);
    vec2 turned = vec2(local.x * c - local.y * s, local.x * s + local.y * c);
    vec2 ndc = (low + 0.5 * side_px + turned) / pc.viewport_px * 2.0 - 1.0;
    // An icon rounded to no pixel covers nothing: its quad collapses.
    if (side_px.x <= 0.0 || side_px.y <= 0.0) {
        ndc = vec2(-2.0);
    }
    gl_Position = vec4(ndc, 0.0, 1.0);

    v_place = vec4(low + 0.5 * side_px, c, s);
    v_side = side_px;
    v_params = in_params;
    v_color0 = in_color0;
    v_color1 = in_color1;
    v_color2 = in_color2;
    v_color3 = in_color3;
    v_color4 = in_color4;
    v_color5 = in_color5;
    v_color6 = in_color6;
    v_color7 = in_color7;
    pass_clip();
}

#version 450
// Instanced straight segment with round caps (a capsule).
//
// One instance per segment, drawn as a 4-vertex triangle strip: a quad aligned with the
// segment, inflated by the half width plus AA_MARGIN physical pixels on every side so the
// fragment shader can compute anti-aliased coverage. Keep the instance layout in sync with
// `LineInstance` (crates/koebi-render/src/scene.rs) and the push constants with
// `ViewportConstants` (crates/koebi-render/src/renderer.rs).

layout(push_constant) uniform ViewportConstants {
    vec2 viewport_px;   // render target size, physical pixels
    float scale_factor; // logical -> physical pixels
    float _padding;
} pc;

layout(location = 0) in vec4 in_points; // from.xy, to.xy (logical px)
layout(location = 1) in vec4 in_color;  // premultiplied, gamma-encoded
layout(location = 2) in vec4 in_params; // x: half width (logical px)

layout(location = 0) out vec2 v_local;            // position relative to `from` (px)
layout(location = 1) flat out vec2 v_segment;     // `to - from` (px)
layout(location = 2) flat out float v_half_width; // px
layout(location = 3) flat out vec4 v_color;

const float AA_MARGIN = 1.0;

void main() {
    // Strip order (0,0) (1,0) (0,1) (1,1): x runs along the segment, y across it.
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));

    vec2 from = in_points.xy * pc.scale_factor;
    vec2 segment = (in_points.zw - in_points.xy) * pc.scale_factor;
    float half_width = in_params.x * pc.scale_factor;

    // A zero-length segment is a dot: any direction works.
    float len = length(segment);
    vec2 along = len > 0.0 ? segment / len : vec2(1.0, 0.0);
    vec2 across = vec2(-along.y, along.x);
    float reach = half_width + AA_MARGIN;
    vec2 local = mix(-along * reach, segment + along * reach, corner.x)
        + across * reach * (corner.y * 2.0 - 1.0);

    // Vulkan clip space: y points down, so the top-left pixel maps to (-1, -1).
    vec2 ndc = (from + local) / pc.viewport_px * 2.0 - 1.0;
    gl_Position = vec4(ndc, 0.0, 1.0);

    v_local = local;
    v_segment = segment;
    v_half_width = half_width;
    v_color = in_color;
}

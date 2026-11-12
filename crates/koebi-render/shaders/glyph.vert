#version 450
#extension GL_GOOGLE_include_directive : require
// Instanced glyph: a quad over the glyph's bounds whose pixels the fragment shader covers from
// the glyph's curves (crates/koebi-render/src/glyphs.rs).
//
// One instance per glyph, drawn as a 4-vertex triangle strip. The em square maps to the screen
// by the pen position and two axes, both in physical pixels: the size, and a rotation, come
// with them. The quad reaches a pixel beyond the bounds, for the coverage of the edge pixels.
// Keep the instance layout in sync with `GlyphInstance` (crates/koebi-render/src/glyphs.rs) and
// the push constants with `ViewportConstants` (crates/koebi-render/src/renderer.rs).

layout(push_constant) uniform ViewportConstants {
    vec2 viewport_px;   // render target size, physical pixels
    float scale_factor; // unused: instances are in physical pixels
    float _padding;
} pc;

#include "clip_vertex.glsl"

layout(location = 0) in vec4 in_origin_axis_x; // pen position, the em's x axis (physical px)
layout(location = 1) in vec4 in_axis_y_record; // the em's y axis, the record's texel, a margin
layout(location = 2) in vec4 in_bounds;        // x min, y min, x max, y max, in ems (y down)
layout(location = 3) in vec4 in_color;         // premultiplied, gamma-encoded

// What maps a pixel to the unit square of the bounds, the same for the whole glyph: nothing is
// interpolated, so every GPU finds the same point for the same pixel.
layout(location = 0) flat out vec4 v_inverse;    // rows of the map from pixels to ems
layout(location = 1) flat out vec4 v_origin_min; // the pen (physical px), the bounds' min (ems)
layout(location = 2) flat out vec4 v_scale_side; // units per em, physical pixels per unit
layout(location = 3) flat out vec4 v_color;
layout(location = 4) flat out float v_record;

void main() {
    // Strip order (0,0) (1,0) (0,1) (1,1) covers the quad with two triangles.
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));
    float margin = in_axis_y_record.w;
    vec2 em = mix(in_bounds.xy - vec2(margin), in_bounds.zw + vec2(margin), corner);
    vec2 position = in_origin_axis_x.xy + em.x * in_origin_axis_x.zw + em.y * in_axis_y_record.xy;
    // Vulkan clip space: y points down, so the top-left pixel maps to (-1, -1).
    gl_Position = vec4(position / pc.viewport_px * 2.0 - 1.0, 0.0, 1.0);
    vec2 size = max(in_bounds.zw - in_bounds.xy, vec2(1e-6));
    // The inverse of the columns (axis x, axis y): pixels back to ems.
    vec2 ax = in_origin_axis_x.zw;
    vec2 ay = in_axis_y_record.xy;
    float det = ax.x * ay.y - ay.x * ax.y;
    float inv = abs(det) > 1e-12 ? 1.0 / det : 0.0;
    v_inverse = vec4(ay.y, -ay.x, -ax.y, ax.x) * inv;
    v_origin_min = vec4(in_origin_axis_x.xy, in_bounds.xy);
    v_scale_side = vec4(1.0 / size, size * vec2(length(ax), length(ay)));
    v_color = in_color;
    v_record = in_axis_y_record.z;
    pass_clip();
}

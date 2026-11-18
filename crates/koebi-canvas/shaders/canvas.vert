#version 450
// One cell of a filled outline: a quad over the cell's own square on the target, whose pixels
// it alone draws (the squares of a grid share their sides exactly: their corners come from the
// CPU in f64). Keep the instance layout in sync with `CellInstance`
// (crates/koebi-canvas/src/canvas.rs).

layout(push_constant) uniform Constants {
    vec2 target_px; // target size, physical pixels
    vec2 _padding;
} pc;

layout(location = 0) in vec4 in_rect;   // x0, y0, x1, y1: the cell's square (px)
layout(location = 1) in vec4 in_map;    // origin.xy (px), pixels per cell side, record
layout(location = 2) in vec4 in_color;  // linear light, premultiplied
layout(location = 3) in vec4 in_params; // x: even-odd, y: margin over the cell's size

layout(location = 0) flat out vec4 v_map;
layout(location = 1) flat out vec4 v_color;
layout(location = 2) flat out vec4 v_params;

void main() {
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));
    vec2 position = mix(in_rect.xy, in_rect.zw, corner);
    // Vulkan clip space: y points down, so the top-left pixel maps to (-1, -1).
    gl_Position = vec4(position / pc.target_px * 2.0 - 1.0, 0.0, 1.0);
    v_map = in_map;
    v_color = in_color;
    v_params = in_params;
}

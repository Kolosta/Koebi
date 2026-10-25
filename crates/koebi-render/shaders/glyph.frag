#version 450
// Glyph coverage from the atlas (one 8-bit channel) applied to the glyph's color.
//
// Coverage multiplies the premultiplied color, for ONE / ONE_MINUS_SRC_ALPHA blending.

layout(set = 0, binding = 0) uniform sampler2D u_atlas;

layout(location = 0) in vec2 v_uv;
layout(location = 1) flat in vec4 v_color;

layout(location = 0) out vec4 out_color;

void main() {
    float coverage = texture(u_atlas, v_uv).r;
    out_color = v_color * coverage;
}

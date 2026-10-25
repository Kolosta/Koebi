#version 450
// Test shader: writes the texture of set 0, binding 0, sampled at the fragment's coordinates.

layout(set = 0, binding = 0) uniform sampler2D u_texture;

layout(location = 0) in vec2 v_uv;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(u_texture, v_uv);
}

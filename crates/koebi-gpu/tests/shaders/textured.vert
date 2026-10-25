#version 450
// Test shader: a quad covering the whole target, with texture coordinates from 0 to 1.
//
// Drawn as a 4-vertex triangle strip without vertex buffers (tests/textures.rs).

layout(location = 0) out vec2 v_uv;

void main() {
    // Strip order (0,0) (1,0) (0,1) (1,1) covers the quad with two triangles.
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));
    v_uv = corner;
    // Vulkan clip space: y points down, so texture row 0 lands on the top row of the target.
    gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}

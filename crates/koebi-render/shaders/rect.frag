#version 450
// Anti-aliased rounded rectangle with optional border, from a signed distance field.
//
// Coverage uses a 1-pixel ramp centered on the edge: pixel-aligned edges stay crisp and
// curved or fractional edges are smooth. Output is premultiplied for ONE / ONE_MINUS_SRC_ALPHA
// blending.

layout(location = 0) in vec2 v_local;
layout(location = 1) flat in vec2 v_half_size;
layout(location = 2) flat in vec4 v_fill;
layout(location = 3) flat in vec4 v_border_color;
layout(location = 4) flat in vec2 v_radius_border;

layout(location = 0) out vec4 out_color;

// Signed distance from p to a rounded box centered at the origin (negative inside).
float sd_rounded_box(vec2 p, vec2 half_size, float radius) {
    vec2 q = abs(p) - half_size + radius;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

void main() {
    float radius = v_radius_border.x;
    float border = v_radius_border.y;

    float dist = sd_rounded_box(v_local, v_half_size, radius);
    float coverage = clamp(0.5 - dist, 0.0, 1.0);

    vec4 color = v_fill;
    if (border > 0.0) {
        float inner = sd_rounded_box(v_local, v_half_size - border, max(radius - border, 0.0));
        color = mix(v_fill, v_border_color, clamp(0.5 + inner, 0.0, 1.0));
    }
    out_color = color * coverage;
}

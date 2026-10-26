#version 450
// Anti-aliased rectangle with a radius per corner and an inner border with a width per side,
// from signed distance fields.
//
// Coverage uses a 1-pixel ramp centered on the edge: pixel-aligned edges stay crisp and
// curved or fractional edges are smooth. Output is premultiplied for ONE / ONE_MINUS_SRC_ALPHA
// blending.

layout(location = 0) in vec2 v_local;
layout(location = 1) flat in vec2 v_half_size;
layout(location = 2) flat in vec4 v_fill;
layout(location = 3) flat in vec4 v_border_color;
layout(location = 4) flat in vec4 v_radii;    // top-left, top-right, bottom-right, bottom-left
layout(location = 5) flat in vec4 v_borders;  // top, right, bottom, left

layout(location = 0) out vec4 out_color;

// The radius of the corner of the quadrant `p` lies in (y points down).
float corner_radius(vec2 p, vec4 radii) {
    return p.x > 0.0 ? (p.y > 0.0 ? radii.z : radii.y) : (p.y > 0.0 ? radii.w : radii.x);
}

// Signed distance from p to a box centered at the origin with a radius per corner (negative
// inside).
float sd_rounded_box(vec2 p, vec2 half_size, vec4 radii) {
    float radius = corner_radius(p, radii);
    vec2 q = abs(p) - half_size + radius;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

void main() {
    float dist = sd_rounded_box(v_local, v_half_size, v_radii);
    float coverage = clamp(0.5 - dist, 0.0, 1.0);

    vec4 color = v_fill;
    float top = v_borders.x;
    float right = v_borders.y;
    float bottom = v_borders.z;
    float left = v_borders.w;
    if (max(max(top, right), max(bottom, left)) > 0.0) {
        // The inner box: the bounds shrunk by each side's width, its corners rounded by what
        // is left of the outer radius past the wider of their two borders.
        vec2 inner_half = max(v_half_size - 0.5 * vec2(left + right, top + bottom), vec2(0.0));
        vec2 inner_center = 0.5 * vec2(left - right, top - bottom);
        vec4 inner_radii = max(
            v_radii - vec4(max(top, left), max(top, right), max(bottom, right), max(bottom, left)),
            vec4(0.0));
        float inner = sd_rounded_box(v_local - inner_center, inner_half, inner_radii);
        color = mix(v_fill, v_border_color, clamp(0.5 + inner, 0.0, 1.0));
    }
    out_color = color * coverage;
}

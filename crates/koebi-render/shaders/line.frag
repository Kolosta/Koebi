#version 450
#extension GL_GOOGLE_include_directive : require
// Anti-aliased segment with round caps, from the signed distance to the segment; two joined
// segments take the distance to the nearer, so that each pixel is covered once.
//
// Same coverage ramp as rectangles: one pixel wide, centered on the edge. Output is
// premultiplied for ONE / ONE_MINUS_SRC_ALPHA blending. A rounded clip cuts the result.

#include "clip.glsl"

layout(location = 0) in vec2 v_local;
layout(location = 1) flat in vec2 v_segment;
layout(location = 2) flat in float v_half_width;
layout(location = 3) flat in vec4 v_color;
layout(location = 4) flat in vec3 v_next;

layout(location = 0) out vec4 out_color;

// Distance from `p` to the segment from `a` to `b` (a dot when they are equal).
float segment_distance(vec2 p, vec2 a, vec2 b) {
    vec2 segment = b - a;
    float length_squared = dot(segment, segment);
    // Closest point of the segment, as a fraction of its length (0 for a dot).
    float t = length_squared > 0.0
        ? clamp(dot(p - a, segment) / length_squared, 0.0, 1.0)
        : 0.0;
    return length(p - a - segment * t);
}

void main() {
    float dist = segment_distance(v_local, vec2(0.0), v_segment);
    if (v_next.z > 0.5) {
        dist = min(dist, segment_distance(v_local, v_segment, v_next.xy));
    }
    dist -= v_half_width;
    float coverage = clamp(0.5 - dist, 0.0, 1.0);
    out_color = v_color * coverage * clip_coverage();
}

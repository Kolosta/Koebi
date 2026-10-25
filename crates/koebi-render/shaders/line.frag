#version 450
// Anti-aliased segment with round caps, from the signed distance to the segment.
//
// Same coverage ramp as rectangles: one pixel wide, centered on the edge. Output is
// premultiplied for ONE / ONE_MINUS_SRC_ALPHA blending.

layout(location = 0) in vec2 v_local;
layout(location = 1) flat in vec2 v_segment;
layout(location = 2) flat in float v_half_width;
layout(location = 3) flat in vec4 v_color;

layout(location = 0) out vec4 out_color;

void main() {
    // Closest point of the segment, as a fraction of its length (0 for a dot).
    float length_squared = dot(v_segment, v_segment);
    float t = length_squared > 0.0
        ? clamp(dot(v_local, v_segment) / length_squared, 0.0, 1.0)
        : 0.0;
    float dist = length(v_local - v_segment * t) - v_half_width;
    float coverage = clamp(0.5 - dist, 0.0, 1.0);
    out_color = v_color * coverage;
}

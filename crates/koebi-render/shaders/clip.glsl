// The rounded clip of a draw, and the signed distance to a rounded box it shares with rectangles.
//
// Included by the fragment shaders. The clip continues `ViewportConstants` of the vertex shaders;
// keep it in sync with `ViewportConstants` (crates/koebi-render/src/renderer.rs).

layout(push_constant) uniform ClipConstants {
    layout(offset = 16) vec4 clip_rect; // low.xy, high.xy (physical px, snapped like rectangles);
                                        // all zero without a rounded clip
    layout(offset = 32) vec4 clip_radii; // top-left, top-right, bottom-right, bottom-left (px)
} clip;

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

// How much of this pixel the rounded clip keeps, with the coverage ramp of rectangles: a corner
// of the clip cuts what is drawn inside exactly where a rectangle of the same shape ends.
float clip_coverage() {
    if (clip.clip_rect.z <= clip.clip_rect.x) {
        return 1.0;
    }
    vec2 half_size = 0.5 * (clip.clip_rect.zw - clip.clip_rect.xy);
    vec2 center = clip.clip_rect.xy + half_size;
    float dist = sd_rounded_box(gl_FragCoord.xy - center, half_size, clip.clip_radii);
    return clamp(0.5 - dist, 0.0, 1.0);
}

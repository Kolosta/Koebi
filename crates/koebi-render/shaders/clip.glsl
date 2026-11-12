// The clip of a fragment, and the signed distance to a rounded box it shares with rectangles.
//
// Included by the fragment shaders. The clip comes from the instance (clip_vertex.glsl): the
// pixels kept, as a scissor keeps them, and the rounded shape cutting them, if any.

layout(location = 13) flat in vec4 v_clip_rect;    // low.xy, high.xy: whole physical pixels
layout(location = 14) flat in vec4 v_clip_rounded; // low.xy, high.xy (physical px, snapped like
                                                   // rectangles); all zero without one
layout(location = 15) flat in vec4 v_clip_radii;   // top-left, top-right, bottom-right,
                                                   // bottom-left (px)

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

// How much of this pixel the clip keeps: none outside its pixels, exactly as a scissor (pixel
// centers lie at half pixels), then the coverage ramp of rectangles at the rounded corners: a
// corner of the clip cuts what is drawn inside exactly where a rectangle of the same shape ends.
float clip_coverage() {
    vec2 p = gl_FragCoord.xy;
    if (any(lessThan(p, v_clip_rect.xy)) || any(greaterThanEqual(p, v_clip_rect.zw))) {
        return 0.0;
    }
    if (v_clip_rounded.z <= v_clip_rounded.x) {
        return 1.0;
    }
    vec2 half_size = 0.5 * (v_clip_rounded.zw - v_clip_rounded.xy);
    vec2 center = v_clip_rounded.xy + half_size;
    float dist = sd_rounded_box(p - center, half_size, v_clip_radii);
    return clamp(0.5 - dist, 0.0, 1.0);
}

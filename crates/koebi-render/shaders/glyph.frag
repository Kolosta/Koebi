#version 450
#extension GL_GOOGLE_include_directive : require
// A glyph's pixel, covered from its curves (crates/koebi-render/src/glyphs.rs).
//
// The glyphs drawn so far are records in a data texture of vec4 texels: a header, the glyph's
// bounds, a table of horizontal bands then vertical bands, and each band's curves in the unit
// square of the bounds. Two rays leave the pixel's center, right and down, as in the icon
// shader: the rightward one crosses the curves of the pixel's horizontal band, the downward one
// those of its vertical band, so a pixel reads a few curves instead of all of them. Glyphs fill
// by the nonzero rule. Output is premultiplied, and a rounded clip cuts it.

#include "clip.glsl"
#include "curves.glsl"

layout(set = 0, binding = 0) uniform sampler2D glyphs;

layout(location = 0) in vec2 v_unit;
layout(location = 1) flat in vec2 v_side;
layout(location = 2) flat in vec4 v_color;
layout(location = 3) flat in float v_record;

layout(location = 0) out vec4 out_color;

// Texels per row of the data texture (DATA_WIDTH in glyphs.rs): a power of two.
const int WIDTH_SHIFT = 12;
// Bounds on what one glyph may read, against corrupt data: far above what glyphs hold.
const int MAX_BANDS = 64;
const int MAX_BAND_CURVES = 16384;

// How many texels the store holds, from its header: no read goes past them.
int g_length = 1;

vec4 fetch(int index) {
    if (uint(index) >= uint(g_length)) {
        return vec4(0.0);
    }
    return texelFetch(glyphs, ivec2(index & ((1 << WIDTH_SHIFT) - 1), index >> WIDTH_SHIFT), 0);
}

void main() {
    // Never past the texture, whatever the header holds.
    ivec2 size = textureSize(glyphs, 0);
    g_length = clamp(int(texelFetch(glyphs, ivec2(0, 0), 0).y), 1, size.x * size.y);
    int record = int(v_record + 0.5);
    // An empty quad, or past the store: nothing (no discard, which slows tiled GPUs down).
    if (record <= 0 || record >= g_length) {
        out_color = vec4(0.0);
        return;
    }
    vec4 head = fetch(record);
    int rows = clamp(int(head.x), 1, MAX_BANDS);
    int columns = clamp(int(head.y), 1, MAX_BANDS);
    // A band holds some of the glyph's curves, never more.
    int curves = clamp(int(head.z), 0, MAX_BAND_CURVES);
    vec2 p = v_unit;
    int row = clamp(int(floor(p.y * float(rows))), 0, rows - 1);
    int column = clamp(int(floor(p.x * float(columns))), 0, columns - 1);
    vec4 across_band = fetch(record + 2 + row);
    vec4 down_band = fetch(record + 2 + rows + column);

    float across = 0.0;
    float across_weight = 0.0;
    int first = int(across_band.x);
    int count = min(int(across_band.y), curves);
    for (int k = 0; k < count; ++k) {
        vec4 ab = fetch(first + 2 * k);
        vec2 a = ab.xy;
        vec2 b = ab.zw;
        vec2 c = fetch(first + 2 * k + 1).xy;
        if (spans(a.y, c.y, p.y)) {
            float t = monotonic_root(a.y, b.y, c.y, p.y);
            float offset = (quad_at(a.x, b.x, c.x, t) - p.x) * v_side.x;
            across += (c.y > a.y ? 1.0 : -1.0) * clamp(offset + 0.5, 0.0, 1.0);
            across_weight = max(across_weight, clamp(1.0 - abs(offset) * 2.0, 0.0, 1.0));
        }
    }
    float down = 0.0;
    float down_weight = 0.0;
    first = int(down_band.x);
    count = min(int(down_band.y), curves);
    for (int k = 0; k < count; ++k) {
        vec4 ab = fetch(first + 2 * k);
        vec2 a = ab.xy;
        vec2 b = ab.zw;
        vec2 c = fetch(first + 2 * k + 1).xy;
        if (spans(a.x, c.x, p.x)) {
            float t = monotonic_root(a.x, b.x, c.x, p.x);
            float offset = (quad_at(a.y, b.y, c.y, t) - p.y) * v_side.y;
            // A curve going left crosses the downward ray as one going down the rightward one.
            down += (c.x < a.x ? 1.0 : -1.0) * clamp(offset + 0.5, 0.0, 1.0);
            down_weight = max(down_weight, clamp(1.0 - abs(offset) * 2.0, 0.0, 1.0));
        }
    }
    across = shade(across, false);
    down = shade(down, false);
    float weights = across_weight + down_weight;
    float mixed = weights > 1e-5 ? (across * across_weight + down * down_weight) / weights : 0.0;
    float cover = clamp(max(mixed, min(across, down)), 0.0, 1.0);
    out_color = v_color * (cover * clip_coverage());
}

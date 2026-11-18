#version 450
#extension GL_GOOGLE_include_directive : require
// A pixel of a cell of a filled outline, covered from the cell's curves (ADR 0029). The
// records lie in a data texture of vec4 texels (crates/koebi-canvas/src/cells.rs); the CPU
// reference (crates/koebi-canvas/src/reference.rs) computes the same in the same order.
//
// Coordinates are the cell's own, 0 to 1 across it, whatever the zoom: the shader's numbers
// stay near 1, which its thresholds expect.
//
// Two rays leave the pixel's center, right and down: the rightward one crosses the curves of
// the pixel's horizontal band and the edges standing for what lies beyond the cell's right side,
// the downward one those of its vertical band and the bottom's edges. Each crossing adds its
// weight, blurred over a pixel; the windings shade under the fill rule and mix by how near each
// ray passes to a curve. The pixel's point comes from gl_FragCoord and flat values, never from
// an interpolated coordinate. Output is linear light, premultiplied.

#include "../../koebi-render/shaders/curves.glsl"

layout(set = 0, binding = 0) uniform sampler2D cells;

layout(location = 0) flat in vec4 v_map;
layout(location = 1) flat in vec4 v_color;
layout(location = 2) flat in vec4 v_params;

layout(location = 0) out vec4 out_color;

// Texels per row of the data texture (DATA_WIDTH in cells.rs): a power of two.
const int WIDTH_SHIFT = 12;
// Bounds on what one cell may read, against corrupt data (MAX_BANDS in cells.rs).
const int MAX_BANDS = 32;
const int MAX_BAND_CURVES = 65536;

int g_length = 1;

vec4 fetch(int index) {
    if (uint(index) >= uint(g_length)) {
        return vec4(0.0);
    }
    return texelFetch(cells, ivec2(index & ((1 << WIDTH_SHIFT) - 1), index >> WIDTH_SHIFT), 0);
}

// The winding of the ray going right (`along` 0) or down (1) through the band at `table` of
// the record at `record`, with how near it passes to a curve.
vec2 ray(int record, int table, int along, vec2 p, int most, float scale, bool even_odd) {
    int other = 1 - along;
    vec4 entry = fetch(table);
    // Counted from the record's start: records move without being rewritten.
    int first = record + int(entry.x);
    int count = min(int(entry.y), most);
    float winding = 0.0;
    float weight = 0.0;
    for (int k = 0; k < count; ++k) {
        vec4 ab = fetch(first + 2 * k);
        vec4 cw = fetch(first + 2 * k + 1);
        vec2 a = ab.xy;
        vec2 b = ab.zw;
        vec2 c = cw.xy;
        if (spans(a[other], c[other], p[other])) {
            float t = monotonic_root(a[other], b[other], c[other], p[other]);
            precise float offset = (quad_at(a[along], b[along], c[along], t) - p[along]) * scale;
            // Going right, a curve going down counts; going down, one going left.
            float sign = along == 0 ? (c.y > a.y ? 1.0 : -1.0) : (c.x < a.x ? 1.0 : -1.0);
            winding += cw.z * sign * clamp(offset + 0.5, 0.0, 1.0);
            weight = max(weight, clamp(1.0 - abs(offset) * 2.0, 0.0, 1.0));
        }
    }
    return vec2(shade(winding, even_odd), weight);
}

void main() {
    ivec2 size = textureSize(cells, 0);
    g_length = clamp(int(texelFetch(cells, ivec2(0, 0), 0).y), 1, size.x * size.y);
    int record = int(v_map.w + 0.5);
    if (record <= 0 || record >= g_length) {
        out_color = vec4(0.0);
        return;
    }
    vec4 head = fetch(record);
    int rows = clamp(int(head.x), 1, MAX_BANDS);
    int columns = clamp(int(head.y), 1, MAX_BANDS);
    int most = clamp(int(head.z), 0, MAX_BAND_CURVES);
    float scale = v_map.z;
    float margin = v_params.y;
    bool even_odd = v_params.x > 0.5;
    // The pixel's center in the cell's coordinates, from its exact position. A product by the
    // inverse the CPU computed, not a quotient: GPUs round products exactly, as the reference
    // does, but may round a quotient 2.5 units off, which moves a point on a curve's extreme
    // off it.
    precise vec2 p = (gl_FragCoord.xy - v_map.xy) * v_params.z;
    float extent = 1.0 + 2.0 * margin;
    int row = clamp(int(floor((p.y + margin) / extent * float(rows))), 0, rows - 1);
    int column = clamp(int(floor((p.x + margin) / extent * float(columns))), 0, columns - 1);
    vec2 across = ray(record, record + 1 + row, 0, p, most, scale, even_odd);
    vec2 down = ray(record, record + 1 + rows + column, 1, p, most, scale, even_odd);
    float weights = across.y + down.y;
    float mixed = weights > 1e-5 ? (across.x * across.y + down.x * down.y) / weights : 0.0;
    float cover = clamp(max(mixed, min(across.x, down.x)), 0.0, 1.0);
    out_color = v_color * cover;
}

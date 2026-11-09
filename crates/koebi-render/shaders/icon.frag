#version 450
#extension GL_GOOGLE_include_directive : require
// A vector icon's pixel, covered from its curves (`IconPrimitive`, crates/koebi-render/src/icon.rs).
//
// The icon set is a data texture of vec4 texels laid out by `koebi_svg::pack` (header, icons,
// zones, layers, curves, stops): keep the reads in sync with crates/koebi-svg/src/pack.rs. The
// coverage is the one `koebi_svg::raster` computes on the CPU (see its module): two rays leave
// the pixel's center, right and down; every curve is monotonic in x and y and crosses each at
// most once, counted over the half-open span of its coordinates, as the share of the pixel on
// the near side of the crossing. The blurred winding along each ray becomes a coverage by the
// fill rule, and the two are mixed by how close their crossings are. Layers are painted over
// each other; output is premultiplied, and a rounded clip cuts it.

#include "clip.glsl"

layout(set = 0, binding = 0) uniform sampler2D icons;

layout(location = 0) in vec2 v_unit;
layout(location = 1) flat in vec2 v_side;
layout(location = 2) flat in vec4 v_params;
layout(location = 3) flat in vec4 v_color0;
layout(location = 4) flat in vec4 v_color1;
layout(location = 5) flat in vec4 v_color2;
layout(location = 6) flat in vec4 v_color3;
layout(location = 7) flat in vec4 v_color4;
layout(location = 8) flat in vec4 v_color5;
layout(location = 9) flat in vec4 v_color6;
layout(location = 10) flat in vec4 v_color7;

layout(location = 0) out vec4 out_color;

// Texels per row of the data texture (DATA_WIDTH in icon.rs): a power of two.
const int WIDTH_SHIFT = 12;
// Bounds on what one icon may read, against a corrupt set: far above what icons hold.
const int MAX_LAYERS = 1024;
const int MAX_CURVES = 65536;
const int MAX_STOPS = 256;

// How many texels the set holds, from its header: no read goes past them.
int g_length = 1;

vec4 fetch(int index) {
    // The renderer validates the set (`Packed::validate`); this guards against the rest.
    if (uint(index) >= uint(g_length)) {
        return vec4(0.0);
    }
    return texelFetch(icons, ivec2(index & ((1 << WIDTH_SHIFT) - 1), index >> WIDTH_SHIFT), 0);
}

vec4 zone_color(int zone) {
    switch (zone) {
        case 0: return v_color0;
        case 1: return v_color1;
        case 2: return v_color2;
        case 3: return v_color3;
        case 4: return v_color4;
        case 5: return v_color5;
        case 6: return v_color6;
        default: return v_color7;
    }
}

// A stop's color (its texels `color` and `meta`), its zone recolored when the instance says so:
// the given color, its alpha multiplied by the stop's own.
vec4 stop_color(vec4 color, vec4 meta, int mask) {
    int zone = int(meta.y);
    if (zone >= 0 && zone < 8 && ((mask >> zone) & 1) != 0) {
        vec4 given = zone_color(zone);
        color = vec4(given.rgb, given.a * color.a);
    }
    return color;
}

float quad_at(float p0, float p1, float p2, float t) {
    float u = 1.0 - t;
    return u * u * p0 + 2.0 * u * t * p1 + t * t * p2;
}

// The parameter where a curve monotonic in one coordinate reaches `y` (monotonic_root in
// raster.rs): the two roots in the form that cancels nothing, the one inside the curve.
float monotonic_root(float y0, float y1, float y2, float y) {
    float a = y0 - 2.0 * y1 + y2;
    float b = 2.0 * (y1 - y0);
    float c = y0 - y;
    float t;
    if (abs(a) < 1e-9) {
        t = abs(b) < 1e-12 ? 0.0 : -c / b;
    } else {
        float discriminant = sqrt(max(b * b - 4.0 * a * c, 0.0));
        float q = -0.5 * (b + (b >= 0.0 ? discriminant : -discriminant));
        float first = q / a;
        float second = abs(q) > 1e-30 ? c / q : first;
        t = (first >= -1e-4 && first <= 1.0001) ? first : second;
    }
    return clamp(t, 0.0, 1.0);
}

bool spans(float from, float to, float value) {
    return value >= min(from, to) && value < max(from, to);
}

// The coverage a blurred winding number gives: nonzero clamps it, even-odd folds it.
float shade(float winding, bool even_odd) {
    return even_odd ? 1.0 - abs(1.0 - mod(abs(winding), 2.0)) : min(abs(winding), 1.0);
}

// The coverage of a layer's curves for the pixel centered on p, `scale` pixels per unit along
// each axis (the square's edges each lie on a pixel edge: its two sides may differ by a pixel).
float coverage(int first, int count, bool even_odd, vec2 p, vec2 scale) {
    float across = 0.0;
    float across_weight = 0.0;
    float down = 0.0;
    float down_weight = 0.0;
    for (int k = 0; k < min(count, MAX_CURVES); ++k) {
        vec4 ab = fetch(first + 2 * k);
        vec2 a = ab.xy;
        vec2 b = ab.zw;
        vec2 c = fetch(first + 2 * k + 1).xy;
        if (spans(a.y, c.y, p.y)) {
            float t = monotonic_root(a.y, b.y, c.y, p.y);
            float offset = (quad_at(a.x, b.x, c.x, t) - p.x) * scale.x;
            across += (c.y > a.y ? 1.0 : -1.0) * clamp(offset + 0.5, 0.0, 1.0);
            across_weight = max(across_weight, clamp(1.0 - abs(offset) * 2.0, 0.0, 1.0));
        }
        if (spans(a.x, c.x, p.x)) {
            float t = monotonic_root(a.x, b.x, c.x, p.x);
            float offset = (quad_at(a.y, b.y, c.y, t) - p.y) * scale.y;
            // A curve going left crosses the downward ray as one going down the rightward one.
            down += (c.x < a.x ? 1.0 : -1.0) * clamp(offset + 0.5, 0.0, 1.0);
            down_weight = max(down_weight, clamp(1.0 - abs(offset) * 2.0, 0.0, 1.0));
        }
    }
    across = shade(across, even_odd);
    down = shade(down, even_odd);
    float weights = across_weight + down_weight;
    float mixed = weights > 1e-5 ? (across * across_weight + down * down_weight) / weights : 0.0;
    return clamp(max(mixed, min(across, down)), 0.0, 1.0);
}

// The straight-alpha color of a layer's paint at p.
vec4 paint(int kind, vec4 transform_x, vec4 transform_y, vec2 p, int mask) {
    int first = int(transform_x.w);
    int count = min(int(transform_y.w), MAX_STOPS);
    vec4 first_meta = fetch(first + 1);
    vec4 previous = stop_color(fetch(first), first_meta, mask);
    if (kind == 0 || count <= 0) {
        return previous;
    }
    vec2 q = vec2(
        transform_x.x * p.x + transform_x.y * p.y + transform_x.z,
        transform_y.x * p.x + transform_y.y * p.y + transform_y.z
    );
    float t = clamp(kind == 2 ? length(q) : q.x, 0.0, 1.0);
    float previous_offset = first_meta.x;
    if (t <= previous_offset) {
        return previous;
    }
    for (int s = 1; s < count; ++s) {
        int stop = first + 2 * s;
        vec4 meta = fetch(stop + 1);
        vec4 next = stop_color(fetch(stop), meta, mask);
        float offset = meta.x;
        if (t <= offset) {
            float share = clamp((t - previous_offset) / max(offset - previous_offset, 1e-6), 0.0, 1.0);
            return mix(previous, next, share);
        }
        previous = next;
        previous_offset = offset;
    }
    return previous;
}

void main() {
    vec4 header = texelFetch(icons, ivec2(0, 0), 0);
    g_length = int(header.y);
    int icon = int(v_params.x);
    // Past the set: nothing (no discard, which slows tiled GPUs down).
    if (float(icon) >= header.x) {
        out_color = vec4(0.0);
        return;
    }
    vec4 record = fetch(1 + icon);
    int first_layer = int(record.x);
    int layers = min(int(record.y), MAX_LAYERS);
    int mask = int(v_params.y);
    vec2 p = v_unit;
    // Half a pixel in the icon's square: a layer farther than that from p leaves it uncovered.
    vec2 reach = 0.5 / v_side;

    vec4 color = vec4(0.0); // premultiplied, layers over each other
    for (int l = 0; l < layers; ++l) {
        int at = first_layer + 4 * l;
        vec4 bounds = fetch(at + 3);
        if (any(lessThan(p, bounds.xy - reach)) || any(greaterThan(p, bounds.zw + reach))) {
            continue;
        }
        vec4 head = fetch(at);
        float cover = coverage(int(head.x), int(head.y), head.z > 0.5, p, v_side);
        if (cover <= 0.0) {
            continue;
        }
        vec4 straight = paint(int(head.w + 0.5), fetch(at + 1), fetch(at + 2), p, mask);
        float alpha = straight.a * cover;
        color = vec4(straight.rgb * alpha, alpha) + color * (1.0 - alpha);
    }
    out_color = color * (v_params.z * clip_coverage());
}

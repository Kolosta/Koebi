#version 450
// A group composited onto the target below it (ADR 0029): the W3C Compositing and Blending
// Level 1 formulas on linear, premultiplied colors, the same as crates/koebi-canvas/src/blend.rs
// in the same order. Targets share their size: each pixel reads its own texel, never filtered.
//
// Normal and Erase leave the compositing to fixed-function blending (over and destination-out):
// the shader writes the source at its opacity and mask. The other modes read the backdrop and
// write the result, unblended, into a target of its own.

layout(set = 0, binding = 0) uniform sampler2D source;
layout(set = 0, binding = 1) uniform sampler2D backdrop;
layout(set = 0, binding = 2) uniform sampler2D mask;

layout(push_constant) uniform Constants {
    uint mode;     // koebi_doc::BlendMode's index
    float opacity; // the source's
    uint flags;    // 1: blend with the backdrop, 2: masked, 4: the mask inverted
    uint _padding;
} pc;

layout(location = 0) out vec4 out_color;

const uint MULTIPLY = 1u;
const uint SCREEN = 2u;
const uint OVERLAY = 3u;
const uint DARKEN = 4u;
const uint LIGHTEN = 5u;
const uint COLOR_DODGE = 6u;
const uint COLOR_BURN = 7u;
const uint HARD_LIGHT = 8u;
const uint SOFT_LIGHT = 9u;
const uint DIFFERENCE = 10u;
const uint EXCLUSION = 11u;
const uint HUE = 12u;
const uint SATURATION = 13u;
const uint COLOR = 14u;
const uint LUMINOSITY = 15u;

float lum(vec3 c) {
    return 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
}

vec3 clip_color(vec3 c) {
    float l = lum(c);
    float n = min(min(c.r, c.g), c.b);
    float x = max(max(c.r, c.g), c.b);
    if (n < 0.0) {
        float d = l - n;
        c = d > 0.0 ? l + (c - l) * l / d : vec3(l);
    }
    if (x > 1.0) {
        float d = x - l;
        c = d > 0.0 ? l + (c - l) * (1.0 - l) / d : vec3(l);
    }
    return c;
}

vec3 set_lum(vec3 c, float l) {
    return clip_color(c + (l - lum(c)));
}

float sat(vec3 c) {
    return max(max(c.r, c.g), c.b) - min(min(c.r, c.g), c.b);
}

vec3 set_sat(vec3 c, float s) {
    float x = max(max(c.r, c.g), c.b);
    float n = min(min(c.r, c.g), c.b);
    return x > n ? (c - n) * s / (x - n) : vec3(0.0);
}

float multiply(float b, float s) {
    return b * s;
}

float screen(float b, float s) {
    return b + s - b * s;
}

float hard_light(float b, float s) {
    return s <= 0.5 ? multiply(b, 2.0 * s) : screen(b, 2.0 * s - 1.0);
}

float separable(uint mode, float b, float s) {
    switch (mode) {
    case MULTIPLY: return multiply(b, s);
    case SCREEN: return screen(b, s);
    case OVERLAY: return hard_light(s, b);
    case DARKEN: return min(b, s);
    case LIGHTEN: return max(b, s);
    case COLOR_DODGE: return b <= 0.0 ? 0.0 : (s >= 1.0 ? 1.0 : min(b / (1.0 - s), 1.0));
    case COLOR_BURN: return b >= 1.0 ? 1.0 : (s <= 0.0 ? 0.0 : 1.0 - min((1.0 - b) / s, 1.0));
    case HARD_LIGHT: return hard_light(b, s);
    case SOFT_LIGHT: {
        if (s <= 0.5) {
            return b - (1.0 - 2.0 * s) * b * (1.0 - b);
        }
        float d = b <= 0.25 ? ((16.0 * b - 12.0) * b + 4.0) * b : sqrt(b);
        return b + (2.0 * s - 1.0) * (d - b);
    }
    case DIFFERENCE: return abs(b - s);
    case EXCLUSION: return b + s - 2.0 * b * s;
    default: return s;
    }
}

vec3 blended(uint mode, vec3 b, vec3 s) {
    switch (mode) {
    case HUE: return set_lum(set_sat(s, sat(b)), lum(b));
    case SATURATION: return set_lum(set_sat(b, sat(s)), lum(b));
    case COLOR: return set_lum(s, lum(b));
    case LUMINOSITY: return set_lum(b, lum(s));
    default:
        return vec3(separable(mode, b.r, s.r), separable(mode, b.g, s.g), separable(mode, b.b, s.b));
    }
}

void main() {
    ivec2 at = ivec2(gl_FragCoord.xy);
    vec4 s = texelFetch(source, at, 0) * pc.opacity;
    if ((pc.flags & 2u) != 0u) {
        float m = texelFetch(mask, at, 0).a;
        s *= (pc.flags & 4u) != 0u ? 1.0 - m : m;
    }
    if ((pc.flags & 1u) == 0u) {
        out_color = s;
        return;
    }
    vec4 b = texelFetch(backdrop, at, 0);
    vec3 cs = s.a > 0.0 ? s.rgb / s.a : vec3(0.0);
    vec3 cb = b.a > 0.0 ? b.rgb / b.a : vec3(0.0);
    vec3 mixed = blended(pc.mode, cb, cs);
    out_color = vec4(s.rgb * (1.0 - b.a) + b.rgb * (1.0 - s.a) + s.a * b.a * mixed,
                     s.a + b.a * (1.0 - s.a));
}

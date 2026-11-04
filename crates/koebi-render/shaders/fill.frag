#version 450
#extension GL_GOOGLE_include_directive : require
// A shape whose color is computed for each pixel: corner colors blended, a checkerboard, strips
// and planes of hues, a hue wheel, a ring and a triangle (`Fill`, crates/koebi-render/src/fill.rs).
//
// Colors are gamma-encoded sRGB, as everywhere in the renderer: HSV and HSL are defined on those
// components, OKLCH converts through linear sRGB with the coefficients of koebi-core, its
// channels clamped (out-of-gamut colors clip as `Oklab::to_color` does). Hue 0 (red) lies at the
// top of wheels and rings, growing clockwise. Edges are anti-aliased from signed distances, a
// one-pixel ramp; output is premultiplied, and a rounded clip cuts it.

#include "clip.glsl"

layout(location = 0) in vec2 v_local;
layout(location = 1) flat in vec2 v_half_size;
layout(location = 2) flat in vec4 v_radii;
layout(location = 3) flat in vec4 v_color0;
layout(location = 4) flat in vec4 v_color1;
layout(location = 5) flat in vec4 v_color2;
layout(location = 6) flat in vec4 v_color3;
layout(location = 7) flat in vec4 v_params;
layout(location = 8) flat in float v_extra;

layout(location = 0) out vec4 out_color;

const float TAU = 6.283185307179586;

// The pure color of `hue` (turns), components 0 to 1.
vec3 pure(float hue) {
    return clamp(abs(fract(hue + vec3(1.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0) - 1.0, 0.0, 1.0);
}

vec3 hsv(float hue, float saturation, float value) {
    return value * mix(vec3(1.0), pure(hue), saturation);
}

vec3 hsl(float hue, float saturation, float lightness) {
    float chroma = (1.0 - abs(2.0 * lightness - 1.0)) * saturation;
    return lightness + chroma * (pure(hue) - 0.5);
}

float encode(float x) {
    x = clamp(x, 0.0, 1.0);
    return x <= 0.0031308 ? 12.92 * x : 1.055 * pow(x, 1.0 / 2.4) - 0.055;
}

vec3 oklch(float lightness, float chroma, float hue) {
    float a = chroma * cos(hue * TAU);
    float b = chroma * sin(hue * TAU);
    float l_ = lightness + 0.3963377774 * a + 0.2158037573 * b;
    float m_ = lightness - 0.1055613458 * a - 0.0638541728 * b;
    float s_ = lightness - 0.0894841775 * a - 1.2914855480 * b;
    float l = l_ * l_ * l_;
    float m = m_ * m_ * m_;
    float s = s_ * s_ * s_;
    vec3 linear = vec3(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s);
    return vec3(encode(linear.r), encode(linear.g), encode(linear.b));
}

// The full-saturation color of `hue` in `model` (0 HSV, 1 HSL, 2 OKLCH at `lightness`, `chroma`).
vec3 hue_color(float model, float hue, float lightness, float chroma) {
    if (model > 1.5) {
        return oklch(lightness, chroma, hue);
    }
    return pure(hue);
}

// The hue of the direction of `p` from the center, in turns: 0 at the top, clockwise. The
// center itself has no direction (`atan(0, 0)` is undefined): it takes hue 0.
float angle_hue(vec2 p) {
    if (dot(p, p) < 1e-12) {
        return 0.0;
    }
    return fract(atan(p.x, -p.y) / TAU + 1.0);
}

// Signed distance to the triangle a, b, c (negative inside).
float sd_triangle(vec2 p, vec2 a, vec2 b, vec2 c) {
    vec2 e0 = b - a, e1 = c - b, e2 = a - c;
    vec2 v0 = p - a, v1 = p - b, v2 = p - c;
    // A degenerate edge (a point) divides by no length: kept above zero.
    vec2 pq0 = v0 - e0 * clamp(dot(v0, e0) / max(dot(e0, e0), 1e-12), 0.0, 1.0);
    vec2 pq1 = v1 - e1 * clamp(dot(v1, e1) / max(dot(e1, e1), 1e-12), 0.0, 1.0);
    vec2 pq2 = v2 - e2 * clamp(dot(v2, e2) / max(dot(e2, e2), 1e-12), 0.0, 1.0);
    float s = sign(e0.x * e2.y - e0.y * e2.x);
    vec2 d = min(min(vec2(dot(pq0, pq0), s * (v0.x * e0.y - v0.y * e0.x)),
                     vec2(dot(pq1, pq1), s * (v1.x * e1.y - v1.y * e1.x))),
                     vec2(dot(pq2, pq2), s * (v2.x * e2.y - v2.y * e2.x)));
    return -sqrt(d.x) * sign(d.y);
}

void main() {
    float kind = v_params.x;
    vec2 uv = clamp(v_local / (2.0 * max(v_half_size, vec2(1e-3))) + 0.5, 0.0, 1.0);
    float radius = min(v_half_size.x, v_half_size.y);
    vec2 p = v_local;
    float dist = sd_rounded_box(p, v_half_size, v_radii);
    vec4 color = vec4(0.0);
    if (kind < 0.5) {
        // Corners: top left, top right, bottom right, bottom left.
        vec4 top = mix(v_color0, v_color1, uv.x);
        vec4 bottom = mix(v_color3, v_color2, uv.x);
        color = mix(top, bottom, uv.y);
    } else if (kind < 1.5) {
        // Checkerboard from the top left.
        vec2 cell = floor((p + v_half_size) / max(v_params.y, 1.0));
        color = mod(cell.x + cell.y, 2.0) < 0.5 ? v_color0 : v_color1;
    } else if (kind < 2.5) {
        // Hues along the length.
        float along = v_params.z > 0.5 ? uv.y : uv.x;
        color = vec4(hue_color(v_params.y, along, v_params.w, v_extra), 1.0);
    } else if (kind < 3.5) {
        // Hue across, saturation up, at a value.
        color = vec4(hsv(uv.x, 1.0 - uv.y, v_params.y), 1.0);
    } else if (kind < 4.5) {
        // A disc: hue around, saturation or chroma outward.
        dist = length(p) - radius;
        float out_t = clamp(length(p) / max(radius, 1e-3), 0.0, 1.0);
        float hue = angle_hue(p);
        vec3 rgb;
        if (v_params.y > 1.5) {
            rgb = oklch(v_params.z, out_t * v_params.w, hue);
        } else if (v_params.y > 0.5) {
            rgb = hsl(hue, out_t, v_params.z);
        } else {
            rgb = hsv(hue, out_t, v_params.z);
        }
        color = vec4(rgb, 1.0);
    } else if (kind < 5.5) {
        // A ring of hues.
        float r = length(p);
        float inner = radius * v_params.z;
        dist = max(r - radius, inner - r);
        color = vec4(hue_color(v_params.y, angle_hue(p), v_params.w, v_extra), 1.0);
    } else if (kind > 6.5) {
        // An arrow: its tip in the middle of the side it points toward (left, right, up,
        // down), outlined inside its edges.
        vec2 h = v_half_size;
        vec2 a, b, c;
        if (v_params.y < 0.5) {
            a = vec2(-h.x, 0.0); b = vec2(h.x, -h.y); c = vec2(h.x, h.y);
        } else if (v_params.y < 1.5) {
            a = vec2(h.x, 0.0); b = vec2(-h.x, h.y); c = vec2(-h.x, -h.y);
        } else if (v_params.y < 2.5) {
            a = vec2(0.0, -h.y); b = vec2(h.x, h.y); c = vec2(-h.x, h.y);
        } else {
            a = vec2(0.0, h.y); b = vec2(-h.x, -h.y); c = vec2(h.x, -h.y);
        }
        dist = sd_triangle(p, a, b, c);
        float edge = clamp(0.5 + dist + v_params.z, 0.0, 1.0);
        color = mix(v_color0, v_color1, v_params.z > 0.0 ? edge : 0.0);
    } else {
        // The triangle of a hue: pure hue, white and black at its corners, clockwise.
        float hue = v_params.y;
        float angle = hue * TAU;
        vec2 a = radius * vec2(sin(angle), -cos(angle));
        vec2 b = radius * vec2(sin(angle + TAU / 3.0), -cos(angle + TAU / 3.0));
        vec2 c = radius * vec2(sin(angle - TAU / 3.0), -cos(angle - TAU / 3.0));
        dist = sd_triangle(p, a, b, c);
        // Barycentric weights, clamped so that the edge's anti-aliasing keeps edge colors.
        vec2 v0 = b - a, v1 = c - a, v2 = p - a;
        float den = v0.x * v1.y - v1.x * v0.y;
        // A triangle with no area (no radius) has no weights: all go to its hue.
        if (abs(den) < 1e-12) {
            den = 1.0;
            v2 = vec2(0.0);
        }
        float wb = (v2.x * v1.y - v1.x * v2.y) / den;
        float wc = (v0.x * v2.y - v2.x * v0.y) / den;
        vec3 w = clamp(vec3(1.0 - wb - wc, wb, wc), 0.0, 1.0);
        w /= max(w.x + w.y + w.z, 1e-5);
        color = vec4(w.x * pure(hue) + w.y * vec3(1.0), 1.0);
    }
    float coverage = clamp(0.5 - dist, 0.0, 1.0);
    out_color = vec4(color.rgb * color.a, color.a) * coverage * clip_coverage();
}

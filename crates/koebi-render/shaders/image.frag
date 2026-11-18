#version 450
#extension GL_GOOGLE_include_directive : require
// An image: its texture read with bilinear filtering, clamped at its edges, anti-aliased at its
// rounded corners like rectangles, its alpha multiplied by its opacity. Texels in linear light
// (premultiplied, the canvas's `Rgba16Float` targets) are encoded to sRGB, the window's
// encoding; gamma-encoded texels are drawn as they are. Output is premultiplied for
// ONE / ONE_MINUS_SRC_ALPHA blending. A rounded clip cuts the result.

#include "clip.glsl"

layout(push_constant) uniform ViewportConstants {
    vec2 viewport_px;
    float scale_factor;
    float linear;
} pc;

layout(set = 0, binding = 0) uniform sampler2D image;

layout(location = 0) in vec2 v_local;
layout(location = 1) in vec2 v_uv;
layout(location = 2) flat in vec2 v_half_size;
layout(location = 3) flat in vec4 v_radii;
layout(location = 4) flat in float v_opacity;

layout(location = 0) out vec4 out_color;

// A linear-light component of 0 to 1, sRGB-encoded.
float encode(float x) {
    x = clamp(x, 0.0, 1.0);
    return x <= 0.0031308 ? 12.92 * x : 1.055 * pow(x, 1.0 / 2.4) - 0.055;
}

void main() {
    float coverage = clamp(0.5 - sd_rounded_box(v_local, v_half_size, v_radii), 0.0, 1.0);
    vec4 texel = texture(image, v_uv);
    float alpha = clamp(texel.a, 0.0, 1.0);
    vec3 rgb = texel.rgb;
    if (pc.linear > 0.5) {
        // Premultiplied linear light to premultiplied sRGB: encode the straight color.
        vec3 straight = alpha > 0.0 ? rgb / alpha : vec3(0.0);
        rgb = vec3(encode(straight.r), encode(straight.g), encode(straight.b)) * alpha;
    }
    out_color = vec4(rgb, alpha) * v_opacity * coverage * clip_coverage();
}

// The clip of an instance: read from its own vertex stream and passed on to the fragment
// shader, which cuts with it (clip.glsl). Every instance carries its clip, so that clips never
// split a batch.
//
// Included by every vertex shader, which calls pass_clip() in main. Keep the stream in sync
// with `ClipInstance` and `CLIP_LOCATION` (crates/koebi-render/src/renderer.rs), and the outputs
// with the inputs of clip.glsl.

layout(location = 12) in vec4 in_clip_rect;    // the pixels kept: low.xy, high.xy (whole px)
layout(location = 13) in vec4 in_clip_rounded; // the rounded clip: low.xy, high.xy (px), all
                                               // zero without one
layout(location = 14) in vec4 in_clip_radii;   // its radii: top-left, top-right, bottom-right,
                                               // bottom-left (px)

layout(location = 13) flat out vec4 v_clip_rect;
layout(location = 14) flat out vec4 v_clip_rounded;
layout(location = 15) flat out vec4 v_clip_radii;

void pass_clip() {
    v_clip_rect = in_clip_rect;
    v_clip_rounded = in_clip_rounded;
    v_clip_radii = in_clip_radii;
}

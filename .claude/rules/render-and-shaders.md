---
paths:
  - "crates/koebi-render/**"
  - "**/*.vert"
  - "**/*.frag"
  - "**/*.comp"
---

# koebi-render and shaders

- Scenes are in logical pixels, origin top-left, y down. Shaders convert to physical pixels
  with the `scale_factor` push constant; keep rectangle edges on whole logical pixels.
- Colors: `Color` is gamma-encoded sRGB with straight alpha; upload `premultiplied()` values.
  Targets are UNORM and blending is `ONE, ONE_MINUS_SRC_ALPHA` (ADR 0004). Never output
  straight alpha from a fragment shader.
- One primitive type = one instance struct (`#[repr(C)]`, `Pod`, size asserted) + one pipeline
  + GLSL pair. Batch consecutive primitives of the same type into one instanced draw. Follow
  the `/render-primitive` skill when adding one.
- Instance layouts and push-constant blocks must match the GLSL exactly; update both sides and
  the comment at the top of the shader together.
- GLSL: `#version 450`, explicit `layout(location = N)` on every input/output, `flat` for
  per-instance data, no implicit derivatives tricks without a comment.
- After editing GLSL run `cargo xtask shaders` and commit the regenerated `shaders/spirv/*.spv`
  (CI rejects stale SPIR-V). Visual changes require `cargo xtask bless` and reviewing the PNG
  diffs in the pull request.
- Per-frame paths must not allocate: grow instance buffers geometrically, reuse the `Scene`.

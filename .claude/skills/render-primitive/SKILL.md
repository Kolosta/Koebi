---
name: render-primitive
description: Add a new 2D primitive to koebi-render (line, circle, gradient, image, glyph run...) with its instance layout, shaders, pipeline, batching and tests.
argument-hint: "<primitive name>"
paths: "crates/koebi-render/**"
---

Add the 2D primitive: $ARGUMENTS

Model it on rectangles (`scene.rs`, `renderer.rs`, `shaders/rect.*`):

1. **Public description** `FooPrimitive` in logical pixels with `koebi_core::Color` fields and
   builder methods (`with_*`), plus `Scene::push_foo`. Cull invisible or empty primitives.
2. **GPU instance** `FooInstance` (`#[repr(C)]`, fields as `[f32; 4]` groups, premultiplied
   colors), `unsafe impl Pod` with `// SAFETY:` and a `const _: () = assert!(size_of::<..>() == N)`.
3. **Shaders** `shaders/foo.vert` / `foo.frag`: same push constants as rect
   (`viewport_px`, `scale_factor`), explicit locations matching the instance layout, analytic
   anti-aliasing (SDF) where possible, premultiplied output. Run `cargo xtask shaders`.
4. **Pipeline** created in `Renderer2D::new` (instance step mode, premultiplied blending),
   embedded with `include_spirv!` in `shaders.rs`.
5. **Batching**: keep painter's order. When several primitive types interleave, record runs
   of identical type as one instanced draw each; never reorder across types.
6. **Streaming**: one host-visible instance buffer per frame in flight, grown geometrically;
   no per-frame allocation.
7. **Tests**: unit tests for instance conversion and culling; functional pixel tests in
   `tests/functional.rs`; a golden scene in `tests/regression.rs` (`cargo xtask bless`,
   review the PNG).
8. Document the primitive in the crate docs and in `docs/architecture/overview.md` if it
   introduces a new rendering technique (and an ADR if it is a significant choice).

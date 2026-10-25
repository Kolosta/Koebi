# Golden images of koebi-render

Reference renders used by [`../regression.rs`](../regression.rs). Each test renders a scene
offscreen on the GPU, reads the pixels back and compares them with the PNG of the same name.
See [testing.md](../../../../docs/development/testing.md#golden-images).

These images test the renderer library, not the application. The scenes are defined in the
tests themselves (`showcase_scene()`, `lines_scene()`), so changing what the application draws
(`apps/koebi/src/gallery/`) never affects them. They fail only when the renderer draws these
scenes differently: a shader, blending, color conversion or scaling change. Intended changes
are blessed and reviewed; unintended ones are regressions. The application is covered by the
end-to-end test, which launches it and checks that it presents frames without validation
errors.

## The showcase scene

A miniature of the application's card plus a translucent overlay that the application does
not draw, chosen to exercise most of the renderer in one image:

| Element | Exercises |
|---|---|
| Background `#111318` | the clear color of the render pass |
| Card: 120x80 at (20, 20), corner radius 16, accent `Oklch(0.62, 0.17, 265)` | OKLCH to sRGB conversion, anti-aliased rounded corners |
| Card border: 2 px, lighter accent | borders drawn inside the bounds |
| Overlay: 56x48 at (90, 60), radius 8, `#FFCC00` at 60 % opacity | premultiplied alpha blending in gamma space, draw order |

Coordinates are logical pixels.

| File | Size | Scale factor | What it adds |
|---|---|---|---|
| `showcase.png` | 160x120 | 1 | the reference rendering |
| `showcase_2x.png` | 320x240 | 2 | the high-DPI path: the same logical scene, twice the pixels |

## The lines scene

Line segments with round caps, over and under rectangles:

| Element | Exercises |
|---|---|
| Check mark on a 24x24 accent box at (12, 12) | two segments sharing an end point (round join), a line over a rectangle |
| Chevron, plus and cross | horizontal, vertical and diagonal anti-aliasing |
| Fan of six segments from (16, 108), widths 0.5 to 4 | hairlines thinner than a pixel, every angle |
| Dots of 1, 2, 4 and 8 px at y = 52 | zero-length segments |
| `#FFCC00` rectangle, then a 6 px accent stroke at 60 % opacity | painter's order across primitive types, blending of strokes |

| File | Size | Scale factor | What it adds |
|---|---|---|---|
| `lines.png` | 160x120 | 1 | the reference rendering |
| `lines_2x.png` | 320x240 | 2 | stroke widths scaled to physical pixels |

## Comparison

`Tolerance::new(3, 8)`: channel differences up to 3/255 are ignored and at most 8 pixels may
differ more, because anti-aliased edges vary slightly between Vulkan drivers (lavapipe,
MoltenVK, desktop GPUs). Every other pixel must match.

## Updating

Never edit these files by hand. After an intended visual change:

```sh
cargo xtask bless              # re-renders every golden image on your GPU
git diff --stat -- '*.png'     # then review the images in the pull request
```

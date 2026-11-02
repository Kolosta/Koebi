# Rectangles and the pixel grid

Interfaces lay out in logical pixels; the screen has physical ones. At a scale of 1.25, a
position of 10.3 logical pixels falls at 12.875 physical pixels: between two pixels.

## What goes wrong without a rule

The rectangle shader computes how much of each pixel a shape covers, from a signed distance
field. That is what makes curves smooth, but it treats a straight edge between two pixels the
same way: the pixel it crosses is partly covered, and shows a color between the two sides.
Three visible defects follow, all from fractional edges:

- one pixel of an intermediate color along the border of a panel;
- a line one logical pixel wide covering 1 pixel here and 2 half-covered pixels there;
- a seam of the background between two rectangles sharing an edge, each covering the pixel
  partly.

## The rule: edges snap, curves do not

Unless it asks otherwise, a [`RectPrimitive`](../../crates/koebi-render/src/scene.rs) is drawn
with [`Edges::Snapped`](../../crates/koebi-render/src/scene.rs). The vertex shader
(`crates/koebi-render/shaders/rect.vert`) does this in physical pixels:

- **Each edge goes to the nearest pixel edge**, not the position and the size separately. Two
  rectangles sharing an edge compute the same pixel for it, so they stay joined at every scale.
  Scissors round the same way, so clipped content meets its container.
  A rounded clip (`Scene::push_rounded_clip`) lies on the same pixels and ends its corners with
  the same coverage as a rectangle of that shape, so an area cuts what it holds exactly at its
  own edge.
- **Thin things keep their thickness.** A rectangle thinner than two pixels is a line: its
  thickness is rounded (at least one pixel) and its start snapped, so that it looks the same
  wherever it lies. Border widths are rounded the same way.
- **Radii stay fractional.** At a pixel center on a snapped straight edge, the distance is
  exactly ±0.5, so coverage is exactly 0 or 1. Only the curve goes through partly covered
  pixels. Nothing else changes in the fragment shader.

This is what interfaces with sharp lines and smooth corners do: straight geometry on integer
pixels, anti-aliasing kept for curves.

A box may still be one pixel wider or narrower depending on where it lies, since both its edges
round separately: that is the price of staying joined to its neighbours, and it cannot be seen
as a defect, only measured.

## Exact edges

[`Edges::Exact`](../../crates/koebi-render/src/scene.rs) keeps edges at their sub-pixel position
and lets the pixels they cross be partly covered. It is the right mode for content that moves or
scales continuously, where snapping would make it jump a pixel at a time: a canvas being zoomed,
for example. Both modes cost the same and can be mixed in a scene.

## Filling outside a shape

`RectPrimitive::inverted` fills a rectangle outside its rounded shape, up to its bounds: only
the outsides of its rounded corners. Previews use it to cover what surrounds a rounded panel
without covering the panel.

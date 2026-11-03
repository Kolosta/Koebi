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

Positions meant to lie exactly half way between two pixels are common (an integer logical
position at 1.25 or 1.75), and floating point cannot hold lengths such as `3 / 1.75` exactly:
such an edge lands a hair below or above the half. Edges therefore round up from
[`TIE_TOLERANCE`](../../crates/koebi-render/src/scene.rs) (1/256 of a pixel) below the half, in
the shader, the scissors and [`koebi_render::pixel_edge`](../../crates/koebi-render/src/scene.rs)
alike, so that edges meant to be a whole number of pixels apart always are.

## Lengths on whole pixels

Edges snap one by one, so a length that is not a whole number of physical pixels comes out a
pixel longer or shorter depending on where it falls: at 1.25, an inset of 2 logical pixels is
2.5 physical pixels, and the controls of a panel header got 3 pixels above and 2 below, or the
reverse; two stacked panels lay 2 or 3 pixels apart. A whole length, on the other hand, keeps
its value wherever it lies: `floor(a + n + 0.5) = floor(a + 0.5) + n`.

So every length that lays components out is a whole number of physical pixels:
[`Styles::resolve_at`](../../crates/koebi-ui/src/style.rs) rounds the heights, insets, gaps,
paddings and indents of every style at the window's scale factor
([`whole_pixels`](../../crates/koebi-ui/src/pixel.rs): the nearest number, ties going down so
that spacing never grows past what the token asks, at least one pixel when the length is not
zero). Strokes, borders, radii and font sizes keep their values. `StyleCache` keeps styles for
each scale factor windows draw at; layouts adding lengths of their own round them with
`Ui::whole_pixels`. Positions may be anything: only the lengths between edges count.

## Exact edges

[`Edges::Exact`](../../crates/koebi-render/src/scene.rs) keeps edges at their sub-pixel position
and lets the pixels they cross be partly covered. It is the right mode for content that moves or
scales continuously, where snapping would make it jump a pixel at a time: a canvas being zoomed,
for example. Both modes cost the same and can be mixed in a scene.

## Filling outside a shape

`RectPrimitive::inverted` fills a rectangle outside its rounded shape, up to its bounds: only
the outsides of its rounded corners. Previews use it to cover what surrounds a rounded panel
without covering the panel.

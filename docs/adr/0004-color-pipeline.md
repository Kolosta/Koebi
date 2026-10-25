# 0004. Color pipeline: sRGB values, gamma-space blending, OKLCH tokens

- Status: Accepted; the token item is superseded by [0008](0008-design-token-system.md)
- Date: 2026-10-24
- Deciders: @Kolosta

## Context

Designers specify colors as sRGB hex values and judge results against design tools and
browsers, which blend legacy sRGB colors in gamma-encoded space. Physically correct blending in
linear light produces visibly different anti-aliased edges, translucent overlays and gradients.
Koebi also needs a design system: color ramps, hover and pressed states, and accessible
contrast, which are hard to build in RGB.

## Decision

- `koebi_core::Color` stores gamma-encoded sRGB with straight alpha, as `f32` components. It is
  what designers type (`Color::from_hex(0x3366FF)`, `Color::from_hex_rgba(0x3366FF80)`).
- Colors are premultiplied when uploaded to the GPU; pipelines blend with premultiplied alpha
  (`ONE`, `ONE_MINUS_SRC_ALPHA`).
- Render targets and swapchains use UNORM formats (`B8G8R8A8_UNORM` first), not `_SRGB`, so
  the GPU blends in gamma space. Pixels match design tools and are exactly testable.
- Design tokens are defined in OKLCH (`Oklch::new(l, c, h).to_color()`), with gamut mapping by
  chroma reduction, so a lightness ramp is perceptually even.
- Theme tests check WCAG contrast ratios (`Color::contrast_ratio`).
- Linear-light work (blur, image scaling, HDR) will use explicit `Rgba16Float` offscreen
  targets and conversions, decided per feature.

## Consequences

- What the designer picks is what the screen shows, including alpha overlays.
- Functional tests compute expected pixels analytically, in 8-bit sRGB.
- Gamma-space blending is not physically linear; effects that need linear light must opt in.
- Wide gamut (Display P3) and HDR output need a future ADR (`VK_EXT_swapchain_colorspace`).

## Alternatives considered

- **`_SRGB` swapchain with linear blending**: physically correct, but results differ from
  design tools and browsers, especially on edges and gradients.
- **Everything in linear `f16`**: double the bandwidth for a UI, and still needs conversion to
  match designers' expectations.
- **HSL tokens**: not perceptually uniform; equal lightness steps look uneven across hues.

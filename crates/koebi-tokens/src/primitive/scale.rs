//! Opacity, spacing, corner radius, stroke width and size scales.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group SCALE: Primitive, start = NEUTRAL::END;

    /// 0 % opacity.
    OPACITY_0 = "opacity.0": Number = 0.0, range(0.0, 1.0);
    /// 10 % opacity.
    OPACITY_100 = "opacity.100": Number = 0.1, range(0.0, 1.0);
    /// 20 % opacity.
    OPACITY_200 = "opacity.200": Number = 0.2, range(0.0, 1.0);
    /// 30 % opacity.
    OPACITY_300 = "opacity.300": Number = 0.3, range(0.0, 1.0);
    /// 40 % opacity.
    OPACITY_400 = "opacity.400": Number = 0.4, range(0.0, 1.0);
    /// 50 % opacity.
    OPACITY_500 = "opacity.500": Number = 0.5, range(0.0, 1.0);
    /// 60 % opacity.
    OPACITY_600 = "opacity.600": Number = 0.6, range(0.0, 1.0);
    /// 70 % opacity.
    OPACITY_700 = "opacity.700": Number = 0.7, range(0.0, 1.0);
    /// 80 % opacity.
    OPACITY_800 = "opacity.800": Number = 0.8, range(0.0, 1.0);
    /// 90 % opacity.
    OPACITY_900 = "opacity.900": Number = 0.9, range(0.0, 1.0);
    /// 100 % opacity.
    OPACITY_1000 = "opacity.1000": Number = 1.0, range(0.0, 1.0);

    /// No spacing.
    SPACING_0 = "spacing.0": Dimension = px(0.0), range(0.0, 256.0);
    /// Spacing scale, step 50.
    SPACING_50 = "spacing.50": Dimension = px(2.0), range(0.0, 256.0);
    /// Spacing scale, step 100.
    SPACING_100 = "spacing.100": Dimension = px(4.0), range(0.0, 256.0);
    /// Spacing scale, step 200.
    SPACING_200 = "spacing.200": Dimension = px(6.0), range(0.0, 256.0);
    /// Spacing scale, step 300.
    SPACING_300 = "spacing.300": Dimension = px(8.0), range(0.0, 256.0);
    /// Spacing scale, step 400.
    SPACING_400 = "spacing.400": Dimension = px(12.0), range(0.0, 256.0);
    /// Spacing scale, step 500.
    SPACING_500 = "spacing.500": Dimension = px(16.0), range(0.0, 256.0);
    /// Spacing scale, step 600.
    SPACING_600 = "spacing.600": Dimension = px(20.0), range(0.0, 256.0);
    /// Spacing scale, step 700.
    SPACING_700 = "spacing.700": Dimension = px(24.0), range(0.0, 256.0);
    /// Spacing scale, step 800.
    SPACING_800 = "spacing.800": Dimension = px(32.0), range(0.0, 256.0);
    /// Spacing scale, step 900.
    SPACING_900 = "spacing.900": Dimension = px(40.0), range(0.0, 256.0);
    /// Spacing scale, step 1000.
    SPACING_1000 = "spacing.1000": Dimension = px(48.0), range(0.0, 256.0);

    /// No rounding: sharp corners.
    RADIUS_NONE = "radius.none": Dimension = px(0.0), range(0.0, 32.0);
    /// Corner radius scale, step 100.
    RADIUS_100 = "radius.100": Dimension = px(2.0), range(0.0, 32.0);
    /// Corner radius scale, step 150.
    RADIUS_150 = "radius.150": Dimension = px(3.0), range(0.0, 32.0);
    /// Corner radius scale, step 200.
    RADIUS_200 = "radius.200": Dimension = px(4.0), range(0.0, 32.0);
    /// Corner radius scale, step 300.
    RADIUS_300 = "radius.300": Dimension = px(6.0), range(0.0, 32.0);
    /// Corner radius scale, step 400.
    RADIUS_400 = "radius.400": Dimension = px(8.0), range(0.0, 32.0);
    /// Corner radius scale, step 500.
    RADIUS_500 = "radius.500": Dimension = px(10.0), range(0.0, 32.0);
    /// Corner radius scale, step 600.
    RADIUS_600 = "radius.600": Dimension = px(12.0), range(0.0, 32.0);
    /// Corner radius scale, step 700.
    RADIUS_700 = "radius.700": Dimension = px(14.0), range(0.0, 32.0);
    /// Corner radius scale, step 800.
    RADIUS_800 = "radius.800": Dimension = px(16.0), range(0.0, 32.0);
    /// Corner radius scale, step 900.
    RADIUS_900 = "radius.900": Dimension = px(20.0), range(0.0, 32.0);
    /// Full rounding: the renderer clamps a radius to half the shortest side, which draws pills and
    /// circles.
    RADIUS_FULL = "radius.full": Dimension = px(9999.0), range(0.0, 9999.0);

    /// No stroke.
    STROKE_WIDTH_0 = "stroke.width.0": Dimension = px(0.0), range(0.0, 4.0);
    /// Stroke width scale, step 100.
    STROKE_WIDTH_100 = "stroke.width.100": Dimension = px(1.0), range(0.0, 4.0);
    /// Stroke width scale, step 150.
    STROKE_WIDTH_150 = "stroke.width.150": Dimension = px(1.5), range(0.0, 4.0);
    /// Stroke width scale, step 200.
    STROKE_WIDTH_200 = "stroke.width.200": Dimension = px(2.0), range(0.0, 4.0);
    /// Stroke width scale, step 300.
    STROKE_WIDTH_300 = "stroke.width.300": Dimension = px(3.0), range(0.0, 4.0);
    /// Stroke width scale, step 400.
    STROKE_WIDTH_400 = "stroke.width.400": Dimension = px(4.0), range(0.0, 4.0);

    /// Size scale, step 75.
    SIZE_75 = "size.75": Dimension = px(12.0), range(0.0, 2048.0);
    /// Size scale, step 100.
    SIZE_100 = "size.100": Dimension = px(22.0), range(0.0, 2048.0);
    /// Size scale, step 200.
    SIZE_200 = "size.200": Dimension = px(32.0), range(0.0, 2048.0);
    /// Size scale, step 400.
    SIZE_400 = "size.400": Dimension = px(80.0), range(0.0, 2048.0);
    /// Size scale, step 800.
    SIZE_800 = "size.800": Dimension = px(180.0), range(0.0, 2048.0);
}

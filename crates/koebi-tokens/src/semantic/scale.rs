//! Interface scale, spacing, size, radius, border width, opacity, shadow and stacking layer scales.

use super::*;
use crate::primitive;

koebi_design::tokens! {
    group SCALE: Semantic, start = DATA_VIZ::END;

    /// Default interface scale factor.
    SCALE_DEFAULT = "scale.default": Number -> primitive::SCALE_100;

    /// Inset of editor content from the editor border.
    SPACING_EDITOR_INSET = "spacing.editor-inset": Dimension -> primitive::SPACING_300;

    /// Size scale: extra-extra-small, for component dimensions.
    SIZE_XXS = "size.xxs": Dimension -> primitive::SIZE_75;
    /// Size scale: extra-small, for component dimensions.
    SIZE_XS = "size.xs": Dimension -> primitive::SIZE_75;
    /// Size scale: small, for component dimensions.
    SIZE_S = "size.s": Dimension -> primitive::SIZE_100;
    /// Size scale: medium, for component dimensions.
    SIZE_M = "size.m": Dimension -> primitive::SIZE_100;
    /// Size scale: large, for component dimensions.
    SIZE_L = "size.l": Dimension -> primitive::SIZE_200;
    /// Size scale: extra-large, for component dimensions.
    SIZE_XL = "size.xl": Dimension -> primitive::SIZE_400;
    /// Size scale: 2xl, for component dimensions.
    SIZE_2XL = "size.2xl": Dimension -> primitive::SIZE_400;
    /// Size scale: 3xl, for component dimensions.
    SIZE_3XL = "size.3xl": Dimension -> primitive::SIZE_800;
    /// Height of standard controls: buttons, fields and dropdowns.
    SIZE_CONTROL_HEIGHT = "size.control-height": Dimension -> primitive::UI_UNIT;

    /// No rounding.
    RADIUS_NONE = "radius.none": Dimension -> primitive::RADIUS_NONE;
    /// Extra-small corner radius: inline badges and tight labels.
    RADIUS_XS = "radius.xs": Dimension -> primitive::RADIUS_100;
    /// Small corner radius: inputs and form fields.
    RADIUS_S = "radius.s": Dimension -> primitive::RADIUS_200;
    /// Medium corner radius: buttons and cards.
    RADIUS_M = "radius.m": Dimension -> primitive::RADIUS_300;
    /// Large corner radius: modals and panels.
    RADIUS_L = "radius.l": Dimension -> primitive::RADIUS_500;
    /// Extra-large corner radius: feature cards.
    RADIUS_XL = "radius.xl": Dimension -> primitive::RADIUS_700;
    /// Full rounding: pills and round avatars.
    RADIUS_FULL = "radius.full": Dimension -> primitive::RADIUS_FULL;
    /// Corner radius of small controls.
    RADIUS_CONTROL = "radius.control": Dimension -> primitive::RADIUS_150;

    /// No border.
    BORDER_WIDTH_NONE = "border.width.none": Dimension -> primitive::STROKE_WIDTH_0;
    /// Thin border width.
    BORDER_WIDTH_THIN = "border.width.thin": Dimension -> primitive::STROKE_WIDTH_100;
    /// Medium border width.
    BORDER_WIDTH_MEDIUM = "border.width.medium": Dimension -> primitive::STROKE_WIDTH_200;
    /// Thick border width.
    BORDER_WIDTH_THICK = "border.width.thick": Dimension -> primitive::STROKE_WIDTH_300;
    /// Width of focus rings.
    BORDER_WIDTH_FOCUS = "border.width.focus": Dimension -> primitive::STROKE_WIDTH_200;

    /// Barely visible opacity.
    OPACITY_GHOST = "opacity.ghost": Number -> primitive::OPACITY_200;
    /// Very reduced opacity.
    OPACITY_FAINT = "opacity.faint": Number -> primitive::OPACITY_300;
    /// Reduced opacity, for de-emphasized content.
    OPACITY_REDUCED = "opacity.reduced": Number -> primitive::OPACITY_400;
    /// Moderate opacity: half dimmed.
    OPACITY_MODERATE = "opacity.moderate": Number -> primitive::OPACITY_600;
    /// Strong opacity: mostly opaque.
    OPACITY_STRONG = "opacity.strong": Number -> primitive::OPACITY_800;
    /// Fully opaque.
    OPACITY_FULL = "opacity.full": Number -> primitive::OPACITY_1000;
    /// Fully transparent.
    OPACITY_TRANSPARENT = "opacity.transparent": Number -> primitive::OPACITY_0;
    /// Default opacity of content.
    OPACITY_DEFAULT = "opacity.default": Number -> primitive::OPACITY_1000;
    /// Opacity of disabled elements.
    OPACITY_DISABLED = "opacity.disabled": Number -> primitive::OPACITY_300;

    /// Light veil over content.
    OPACITY_DIM_LIGHT = "opacity.dim.light": Number -> OPACITY_MODERATE;
    /// Heavy veil over content.
    OPACITY_DIM_HEAVY = "opacity.dim.heavy": Number -> OPACITY_STRONG;

    /// Opacity of disabled content.
    OPACITY_CONTENT_DISABLED = "opacity.content.disabled": Number -> OPACITY_REDUCED;

    /// Shadow color, elevation step 100.
    SHADOW_100_COLOR = "shadow.100.color": Color -> primitive::COLOR_TRANSPARENT_BLACK_100;

    /// Shadow color, elevation step 200.
    SHADOW_200_COLOR = "shadow.200.color": Color -> primitive::COLOR_TRANSPARENT_BLACK_200;

    /// Shadow color, elevation step 300.
    SHADOW_300_COLOR = "shadow.300.color": Color -> primitive::COLOR_TRANSPARENT_BLACK_300;

    /// Shadow color, elevation step 400.
    SHADOW_400_COLOR = "shadow.400.color": Color -> primitive::COLOR_TRANSPARENT_BLACK_300;

    /// Shadow color, elevation step 500.
    SHADOW_500_COLOR = "shadow.500.color": Color -> primitive::COLOR_TRANSPARENT_BLACK_300;

    /// Shadow color, elevation step 600.
    SHADOW_600_COLOR = "shadow.600.color": Color -> primitive::COLOR_TRANSPARENT_BLACK_300;

    /// Shadow color of barely lifted, flat surfaces.
    SHADOW_XS = "shadow.xs": Color -> SHADOW_100_COLOR;
    /// Shadow color of cards and inputs.
    SHADOW_S = "shadow.s": Color -> SHADOW_200_COLOR;
    /// Shadow color of dropdowns and popovers.
    SHADOW_M = "shadow.m": Color -> SHADOW_300_COLOR;
    /// Shadow color of dialogs.
    SHADOW_L = "shadow.l": Color -> SHADOW_400_COLOR;
    /// Shadow color of side panels and drawers.
    SHADOW_XL = "shadow.xl": Color -> SHADOW_500_COLOR;

    /// Stacking layer of dropdowns.
    LAYER_DROPDOWN = "layer.dropdown": Integer -> primitive::LAYER_HIGH;
    /// Stacking layer of stickys.
    LAYER_STICKY = "layer.sticky": Integer -> primitive::LAYER_MID;
    /// Stacking layer of overlays.
    LAYER_OVERLAY = "layer.overlay": Integer -> primitive::LAYER_VEIL;
    /// Stacking layer of modals.
    LAYER_MODAL = "layer.modal": Integer -> primitive::LAYER_RAISED;
    /// Stacking layer of popovers.
    LAYER_POPOVER = "layer.popover": Integer -> primitive::LAYER_PEAK;
    /// Stacking layer of toasts.
    LAYER_TOAST = "layer.toast": Integer -> primitive::LAYER_CRITICAL;
    /// Stacking layer of tooltips.
    LAYER_TOOLTIP = "layer.tooltip": Integer -> primitive::LAYER_ABSOLUTE;
}

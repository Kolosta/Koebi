//! Neutral colors: the gray scale (25 is white, 1000 is black), transparent black and white, and
//! static colors that never change with the theme.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group NEUTRAL: Primitive, start = PALETTE::END;

    /// Gray, palette step 25 (white).
    COLOR_GRAY_25 = "color.gray.25": Color = hex("#FFFFFF");
    /// Gray, palette step 50.
    COLOR_GRAY_50 = "color.gray.50": Color = hex("#F8F8F8");
    /// Gray, palette step 75.
    COLOR_GRAY_75 = "color.gray.75": Color = hex("#F3F3F3");
    /// Gray, palette step 100.
    COLOR_GRAY_100 = "color.gray.100": Color = hex("#E9E9E9");
    /// Gray, palette step 200.
    COLOR_GRAY_200 = "color.gray.200": Color = hex("#E1E1E1");
    /// Gray, palette step 300.
    COLOR_GRAY_300 = "color.gray.300": Color = hex("#DADADA");
    /// Gray, palette step 400.
    COLOR_GRAY_400 = "color.gray.400": Color = hex("#C6C6C6");
    /// Gray, palette step 500.
    COLOR_GRAY_500 = "color.gray.500": Color = hex("#8F8F8F");
    /// Gray, palette step 600.
    COLOR_GRAY_600 = "color.gray.600": Color = hex("#717171");
    /// Gray, palette step 700.
    COLOR_GRAY_700 = "color.gray.700": Color = hex("#505050");
    /// Gray, palette step 740.
    COLOR_GRAY_740 = "color.gray.740": Color = hex("#484848");
    /// Gray, palette step 760.
    COLOR_GRAY_760 = "color.gray.760": Color = hex("#3C3C3C");
    /// Gray, palette step 780.
    COLOR_GRAY_780 = "color.gray.780": Color = hex("#323232");
    /// Gray, palette step 800.
    COLOR_GRAY_800 = "color.gray.800": Color = hex("#292929");
    /// Gray, palette step 850.
    COLOR_GRAY_850 = "color.gray.850": Color = hex("#1E1E1E");
    /// Gray, palette step 875.
    COLOR_GRAY_875 = "color.gray.875": Color = hex("#232323");
    /// Gray, palette step 900.
    COLOR_GRAY_900 = "color.gray.900": Color = hex("#131313");
    /// Gray, palette step 1000 (black).
    COLOR_GRAY_1000 = "color.gray.1000": Color = hex("#000000");

    /// Black, fully transparent.
    COLOR_TRANSPARENT_BLACK_25 = "color.transparent-black.25": Color = hex("#00000000");
    /// Black at 3 % opacity.
    COLOR_TRANSPARENT_BLACK_50 = "color.transparent-black.50": Color = hex("#00000008");
    /// Black at 5 % opacity.
    COLOR_TRANSPARENT_BLACK_75 = "color.transparent-black.75": Color = hex("#0000000D");
    /// Black at 9 % opacity.
    COLOR_TRANSPARENT_BLACK_100 = "color.transparent-black.100": Color = hex("#00000017");
    /// Black at 12 % opacity.
    COLOR_TRANSPARENT_BLACK_200 = "color.transparent-black.200": Color = hex("#0000001F");
    /// Black at 15 % opacity.
    COLOR_TRANSPARENT_BLACK_300 = "color.transparent-black.300": Color = hex("#00000026");
    /// Black at 22 % opacity.
    COLOR_TRANSPARENT_BLACK_400 = "color.transparent-black.400": Color = hex("#00000038");
    /// Black at 44 % opacity.
    COLOR_TRANSPARENT_BLACK_500 = "color.transparent-black.500": Color = hex("#00000070");
    /// Black at 56 % opacity.
    COLOR_TRANSPARENT_BLACK_600 = "color.transparent-black.600": Color = hex("#0000008F");
    /// Black at 69 % opacity.
    COLOR_TRANSPARENT_BLACK_700 = "color.transparent-black.700": Color = hex("#000000B0");
    /// Black at 84 % opacity.
    COLOR_TRANSPARENT_BLACK_800 = "color.transparent-black.800": Color = hex("#000000D6");
    /// Black at 93 % opacity.
    COLOR_TRANSPARENT_BLACK_900 = "color.transparent-black.900": Color = hex("#000000ED");
    /// Black at 100 % opacity.
    COLOR_TRANSPARENT_BLACK_1000 = "color.transparent-black.1000": Color = hex("#000000");

    /// White, fully transparent.
    COLOR_TRANSPARENT_WHITE_25 = "color.transparent-white.25": Color = hex("#FFFFFF00");
    /// White at 4 % opacity.
    COLOR_TRANSPARENT_WHITE_50 = "color.transparent-white.50": Color = hex("#FFFFFF0A");
    /// White at 7 % opacity.
    COLOR_TRANSPARENT_WHITE_75 = "color.transparent-white.75": Color = hex("#FFFFFF12");
    /// White at 11 % opacity.
    COLOR_TRANSPARENT_WHITE_100 = "color.transparent-white.100": Color = hex("#FFFFFF1C");
    /// White at 14 % opacity.
    COLOR_TRANSPARENT_WHITE_200 = "color.transparent-white.200": Color = hex("#FFFFFF24");
    /// White at 17 % opacity.
    COLOR_TRANSPARENT_WHITE_300 = "color.transparent-white.300": Color = hex("#FFFFFF2B");
    /// White at 21 % opacity.
    COLOR_TRANSPARENT_WHITE_400 = "color.transparent-white.400": Color = hex("#FFFFFF36");
    /// White at 39 % opacity.
    COLOR_TRANSPARENT_WHITE_500 = "color.transparent-white.500": Color = hex("#FFFFFF63");
    /// White at 51 % opacity.
    COLOR_TRANSPARENT_WHITE_600 = "color.transparent-white.600": Color = hex("#FFFFFF82");
    /// White at 66 % opacity.
    COLOR_TRANSPARENT_WHITE_700 = "color.transparent-white.700": Color = hex("#FFFFFFA8");
    /// White at 85 % opacity.
    COLOR_TRANSPARENT_WHITE_800 = "color.transparent-white.800": Color = hex("#FFFFFFD9");
    /// White at 94 % opacity.
    COLOR_TRANSPARENT_WHITE_900 = "color.transparent-white.900": Color = hex("#FFFFFFF0");
    /// White at 100 % opacity.
    COLOR_TRANSPARENT_WHITE_1000 = "color.transparent-white.1000": Color = hex("#FFFFFF");

    /// White at 18 % opacity, for drop previews.
    COLOR_WHITE_TRANSPARENT = "color.white-transparent": Color = hex("#FFFFFF2E");

    /// Static blue, step 900: identical in every theme.
    COLOR_STATIC_BLUE_900 = "color.static.blue.900": Color = hex("#3B63FB");
    /// Static blue, step 1000: identical in every theme.
    COLOR_STATIC_BLUE_1000 = "color.static.blue.1000": Color = hex("#274DEA");
    /// Static red, step 400: identical in every theme.
    COLOR_STATIC_RED_400 = "color.static.red.400": Color = hex("#FFBCB4");
    /// Static red, step 600: identical in every theme.
    COLOR_STATIC_RED_600 = "color.static.red.600": Color = hex("#FF7665");
    /// Static red, step 800: identical in every theme.
    COLOR_STATIC_RED_800 = "color.static.red.800": Color = hex("#F03823");
    /// Static red, step 900: identical in every theme.
    COLOR_STATIC_RED_900 = "color.static.red.900": Color = hex("#D73220");
    /// Static red, step 1000: identical in every theme.
    COLOR_STATIC_RED_1000 = "color.static.red.1000": Color = hex("#B72818");
    /// Static green, step 400: identical in every theme.
    COLOR_STATIC_GREEN_400 = "color.static.green.400": Color = hex("#6BE3A2");
    /// Static green, step 600: identical in every theme.
    COLOR_STATIC_GREEN_600 = "color.static.green.600": Color = hex("#12B867");
    /// Static green, step 800: identical in every theme.
    COLOR_STATIC_GREEN_800 = "color.static.green.800": Color = hex("#079355");
    /// Static orange, step 400: identical in every theme.
    COLOR_STATIC_ORANGE_400 = "color.static.orange.400": Color = hex("#FFC15E");
    /// Static orange, step 600: identical in every theme.
    COLOR_STATIC_ORANGE_600 = "color.static.orange.600": Color = hex("#FC7D00");
    /// Static orange, step 800: identical in every theme.
    COLOR_STATIC_ORANGE_800 = "color.static.orange.800": Color = hex("#D45B00");
}

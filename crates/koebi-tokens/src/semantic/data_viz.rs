//! Data visualization colors: categorical, ordinal, sequential and diverging scales.

use super::*;
use crate::primitive;

koebi_design::tokens! {
    group DATA_VIZ: Semantic, start = CONTENT::END;

    /// Categorical data color 1 of 6, distinguishable with color vision deficiencies.
    DATA_VIZ_CATEGORICAL_SHORT_1 = "data-viz.categorical.short.1": Color ->
        primitive::COLOR_STATIC_BLUE_900;
    /// Categorical data color 2 of 6, distinguishable with color vision deficiencies.
    DATA_VIZ_CATEGORICAL_SHORT_2 = "data-viz.categorical.short.2": Color ->
        primitive::COLOR_STATIC_ORANGE_600;
    /// Categorical data color 3 of 6, distinguishable with color vision deficiencies.
    DATA_VIZ_CATEGORICAL_SHORT_3 = "data-viz.categorical.short.3": Color ->
        primitive::COLOR_STATIC_GREEN_600;
    /// Categorical data color 4 of 6, distinguishable with color vision deficiencies.
    DATA_VIZ_CATEGORICAL_SHORT_4 = "data-viz.categorical.short.4": Color ->
        primitive::COLOR_STATIC_RED_800;
    /// Categorical data color 5 of 6, distinguishable with color vision deficiencies.
    DATA_VIZ_CATEGORICAL_SHORT_5 = "data-viz.categorical.short.5": Color ->
        primitive::COLOR_STATIC_BLUE_1000;
    /// Categorical data color 6 of 6, distinguishable with color vision deficiencies.
    DATA_VIZ_CATEGORICAL_SHORT_6 = "data-viz.categorical.short.6": Color ->
        primitive::COLOR_STATIC_ORANGE_800;

    /// Categorical data color 7 of the extended 12-color palette.
    DATA_VIZ_CATEGORICAL_EXTENDED_7 = "data-viz.categorical.extended.7": Color ->
        primitive::COLOR_STATIC_GREEN_400;
    /// Categorical data color 8 of the extended 12-color palette.
    DATA_VIZ_CATEGORICAL_EXTENDED_8 = "data-viz.categorical.extended.8": Color ->
        primitive::COLOR_STATIC_RED_400;
    /// Categorical data color 9 of the extended 12-color palette.
    DATA_VIZ_CATEGORICAL_EXTENDED_9 = "data-viz.categorical.extended.9": Color ->
        primitive::COLOR_STATIC_RED_600;
    /// Categorical data color 10 of the extended 12-color palette.
    DATA_VIZ_CATEGORICAL_EXTENDED_10 = "data-viz.categorical.extended.10": Color ->
        primitive::COLOR_STATIC_ORANGE_400;
    /// Categorical data color 11 of the extended 12-color palette.
    DATA_VIZ_CATEGORICAL_EXTENDED_11 = "data-viz.categorical.extended.11": Color ->
        primitive::COLOR_STATIC_GREEN_800;
    /// Categorical data color 12 of the extended 12-color palette.
    DATA_VIZ_CATEGORICAL_EXTENDED_12 = "data-viz.categorical.extended.12": Color ->
        primitive::COLOR_STATIC_BLUE_900;

    /// Ordinal data color 1 of 9, from light to dark.
    DATA_VIZ_ORDINAL_1 = "data-viz.ordinal.1": Color -> primitive::COLOR_BLUE_100;
    /// Ordinal data color 2 of 9, from light to dark.
    DATA_VIZ_ORDINAL_2 = "data-viz.ordinal.2": Color -> primitive::COLOR_BLUE_300;
    /// Ordinal data color 3 of 9, from light to dark.
    DATA_VIZ_ORDINAL_3 = "data-viz.ordinal.3": Color -> primitive::COLOR_BLUE_500;
    /// Ordinal data color 4 of 9, from light to dark.
    DATA_VIZ_ORDINAL_4 = "data-viz.ordinal.4": Color -> primitive::COLOR_BLUE_700;
    /// Ordinal data color 5 of 9, from light to dark.
    DATA_VIZ_ORDINAL_5 = "data-viz.ordinal.5": Color -> primitive::COLOR_BLUE_800;
    /// Ordinal data color 6 of 9, from light to dark.
    DATA_VIZ_ORDINAL_6 = "data-viz.ordinal.6": Color -> primitive::COLOR_BLUE_900;
    /// Ordinal data color 7 of 9, from light to dark.
    DATA_VIZ_ORDINAL_7 = "data-viz.ordinal.7": Color -> primitive::COLOR_BLUE_1100;
    /// Ordinal data color 8 of 9, from light to dark.
    DATA_VIZ_ORDINAL_8 = "data-viz.ordinal.8": Color -> primitive::COLOR_BLUE_1300;
    /// Ordinal data color 9 of 9, from light to dark.
    DATA_VIZ_ORDINAL_9 = "data-viz.ordinal.9": Color -> primitive::COLOR_BLUE_1500;

    /// Sequential viridis scale, step 100.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_100 = "data-viz.sequential.viridis.100": Color ->
        primitive::COLOR_GREEN_100;
    /// Sequential viridis scale, step 200.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_200 = "data-viz.sequential.viridis.200": Color ->
        primitive::COLOR_GREEN_200;
    /// Sequential viridis scale, step 300.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_300 = "data-viz.sequential.viridis.300": Color ->
        primitive::COLOR_GREEN_300;
    /// Sequential viridis scale, step 400.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_400 = "data-viz.sequential.viridis.400": Color ->
        primitive::COLOR_GREEN_400;
    /// Sequential viridis scale, step 500.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_500 = "data-viz.sequential.viridis.500": Color ->
        primitive::COLOR_GREEN_500;
    /// Sequential viridis scale, step 600.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_600 = "data-viz.sequential.viridis.600": Color ->
        primitive::COLOR_GREEN_600;
    /// Sequential viridis scale, step 700.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_700 = "data-viz.sequential.viridis.700": Color ->
        primitive::COLOR_GREEN_700;
    /// Sequential viridis scale, step 800.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_800 = "data-viz.sequential.viridis.800": Color ->
        primitive::COLOR_GREEN_800;
    /// Sequential viridis scale, step 900.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_900 = "data-viz.sequential.viridis.900": Color ->
        primitive::COLOR_GREEN_900;
    /// Sequential viridis scale, step 1000.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_1000 = "data-viz.sequential.viridis.1000": Color ->
        primitive::COLOR_GREEN_1000;
    /// Sequential viridis scale, step 1100.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_1100 = "data-viz.sequential.viridis.1100": Color ->
        primitive::COLOR_GREEN_1100;
    /// Sequential viridis scale, step 1200.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_1200 = "data-viz.sequential.viridis.1200": Color ->
        primitive::COLOR_GREEN_1200;
    /// Sequential viridis scale, step 1300.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_1300 = "data-viz.sequential.viridis.1300": Color ->
        primitive::COLOR_GREEN_1300;
    /// Sequential viridis scale, step 1400.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_1400 = "data-viz.sequential.viridis.1400": Color ->
        primitive::COLOR_GREEN_1400;
    /// Sequential viridis scale, step 1500.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_1500 = "data-viz.sequential.viridis.1500": Color ->
        primitive::COLOR_GREEN_1500;
    /// Sequential viridis scale, step 1600.
    DATA_VIZ_SEQUENTIAL_VIRIDIS_1600 = "data-viz.sequential.viridis.1600": Color ->
        primitive::COLOR_GREEN_1600;

    /// Sequential magma scale, step 100.
    DATA_VIZ_SEQUENTIAL_MAGMA_100 = "data-viz.sequential.magma.100": Color ->
        primitive::COLOR_MAGENTA_100;
    /// Sequential magma scale, step 200.
    DATA_VIZ_SEQUENTIAL_MAGMA_200 = "data-viz.sequential.magma.200": Color ->
        primitive::COLOR_MAGENTA_200;
    /// Sequential magma scale, step 300.
    DATA_VIZ_SEQUENTIAL_MAGMA_300 = "data-viz.sequential.magma.300": Color ->
        primitive::COLOR_MAGENTA_300;
    /// Sequential magma scale, step 400.
    DATA_VIZ_SEQUENTIAL_MAGMA_400 = "data-viz.sequential.magma.400": Color ->
        primitive::COLOR_MAGENTA_400;
    /// Sequential magma scale, step 500.
    DATA_VIZ_SEQUENTIAL_MAGMA_500 = "data-viz.sequential.magma.500": Color ->
        primitive::COLOR_MAGENTA_500;
    /// Sequential magma scale, step 600.
    DATA_VIZ_SEQUENTIAL_MAGMA_600 = "data-viz.sequential.magma.600": Color ->
        primitive::COLOR_MAGENTA_600;
    /// Sequential magma scale, step 700.
    DATA_VIZ_SEQUENTIAL_MAGMA_700 = "data-viz.sequential.magma.700": Color ->
        primitive::COLOR_MAGENTA_700;
    /// Sequential magma scale, step 800.
    DATA_VIZ_SEQUENTIAL_MAGMA_800 = "data-viz.sequential.magma.800": Color ->
        primitive::COLOR_MAGENTA_800;
    /// Sequential magma scale, step 900.
    DATA_VIZ_SEQUENTIAL_MAGMA_900 = "data-viz.sequential.magma.900": Color ->
        primitive::COLOR_MAGENTA_900;
    /// Sequential magma scale, step 1000.
    DATA_VIZ_SEQUENTIAL_MAGMA_1000 = "data-viz.sequential.magma.1000": Color ->
        primitive::COLOR_MAGENTA_1000;
    /// Sequential magma scale, step 1100.
    DATA_VIZ_SEQUENTIAL_MAGMA_1100 = "data-viz.sequential.magma.1100": Color ->
        primitive::COLOR_MAGENTA_1100;
    /// Sequential magma scale, step 1200.
    DATA_VIZ_SEQUENTIAL_MAGMA_1200 = "data-viz.sequential.magma.1200": Color ->
        primitive::COLOR_MAGENTA_1200;
    /// Sequential magma scale, step 1300.
    DATA_VIZ_SEQUENTIAL_MAGMA_1300 = "data-viz.sequential.magma.1300": Color ->
        primitive::COLOR_MAGENTA_1300;
    /// Sequential magma scale, step 1400.
    DATA_VIZ_SEQUENTIAL_MAGMA_1400 = "data-viz.sequential.magma.1400": Color ->
        primitive::COLOR_MAGENTA_1400;
    /// Sequential magma scale, step 1500.
    DATA_VIZ_SEQUENTIAL_MAGMA_1500 = "data-viz.sequential.magma.1500": Color ->
        primitive::COLOR_MAGENTA_1500;
    /// Sequential magma scale, step 1600.
    DATA_VIZ_SEQUENTIAL_MAGMA_1600 = "data-viz.sequential.magma.1600": Color ->
        primitive::COLOR_MAGENTA_1600;

    /// Sequential plasma scale, step 100.
    DATA_VIZ_SEQUENTIAL_PLASMA_100 = "data-viz.sequential.plasma.100": Color ->
        primitive::COLOR_PURPLE_100;
    /// Sequential plasma scale, step 200.
    DATA_VIZ_SEQUENTIAL_PLASMA_200 = "data-viz.sequential.plasma.200": Color ->
        primitive::COLOR_PURPLE_200;
    /// Sequential plasma scale, step 300.
    DATA_VIZ_SEQUENTIAL_PLASMA_300 = "data-viz.sequential.plasma.300": Color ->
        primitive::COLOR_PURPLE_300;
    /// Sequential plasma scale, step 400.
    DATA_VIZ_SEQUENTIAL_PLASMA_400 = "data-viz.sequential.plasma.400": Color ->
        primitive::COLOR_PURPLE_400;
    /// Sequential plasma scale, step 500.
    DATA_VIZ_SEQUENTIAL_PLASMA_500 = "data-viz.sequential.plasma.500": Color ->
        primitive::COLOR_PURPLE_500;
    /// Sequential plasma scale, step 600.
    DATA_VIZ_SEQUENTIAL_PLASMA_600 = "data-viz.sequential.plasma.600": Color ->
        primitive::COLOR_PURPLE_600;
    /// Sequential plasma scale, step 700.
    DATA_VIZ_SEQUENTIAL_PLASMA_700 = "data-viz.sequential.plasma.700": Color ->
        primitive::COLOR_PURPLE_700;
    /// Sequential plasma scale, step 800.
    DATA_VIZ_SEQUENTIAL_PLASMA_800 = "data-viz.sequential.plasma.800": Color ->
        primitive::COLOR_PURPLE_800;
    /// Sequential plasma scale, step 900.
    DATA_VIZ_SEQUENTIAL_PLASMA_900 = "data-viz.sequential.plasma.900": Color ->
        primitive::COLOR_PURPLE_900;
    /// Sequential plasma scale, step 1000.
    DATA_VIZ_SEQUENTIAL_PLASMA_1000 = "data-viz.sequential.plasma.1000": Color ->
        primitive::COLOR_PURPLE_1000;
    /// Sequential plasma scale, step 1100.
    DATA_VIZ_SEQUENTIAL_PLASMA_1100 = "data-viz.sequential.plasma.1100": Color ->
        primitive::COLOR_PURPLE_1100;
    /// Sequential plasma scale, step 1200.
    DATA_VIZ_SEQUENTIAL_PLASMA_1200 = "data-viz.sequential.plasma.1200": Color ->
        primitive::COLOR_PURPLE_1200;
    /// Sequential plasma scale, step 1300.
    DATA_VIZ_SEQUENTIAL_PLASMA_1300 = "data-viz.sequential.plasma.1300": Color ->
        primitive::COLOR_PURPLE_1300;
    /// Sequential plasma scale, step 1400.
    DATA_VIZ_SEQUENTIAL_PLASMA_1400 = "data-viz.sequential.plasma.1400": Color ->
        primitive::COLOR_PURPLE_1400;
    /// Sequential plasma scale, step 1500.
    DATA_VIZ_SEQUENTIAL_PLASMA_1500 = "data-viz.sequential.plasma.1500": Color ->
        primitive::COLOR_PURPLE_1500;
    /// Sequential plasma scale, step 1600.
    DATA_VIZ_SEQUENTIAL_PLASMA_1600 = "data-viz.sequential.plasma.1600": Color ->
        primitive::COLOR_PURPLE_1600;

    /// Sequential inferno scale, step 100.
    DATA_VIZ_SEQUENTIAL_INFERNO_100 = "data-viz.sequential.inferno.100": Color ->
        primitive::COLOR_ORANGE_100;
    /// Sequential inferno scale, step 200.
    DATA_VIZ_SEQUENTIAL_INFERNO_200 = "data-viz.sequential.inferno.200": Color ->
        primitive::COLOR_ORANGE_200;
    /// Sequential inferno scale, step 300.
    DATA_VIZ_SEQUENTIAL_INFERNO_300 = "data-viz.sequential.inferno.300": Color ->
        primitive::COLOR_ORANGE_300;
    /// Sequential inferno scale, step 400.
    DATA_VIZ_SEQUENTIAL_INFERNO_400 = "data-viz.sequential.inferno.400": Color ->
        primitive::COLOR_ORANGE_400;
    /// Sequential inferno scale, step 500.
    DATA_VIZ_SEQUENTIAL_INFERNO_500 = "data-viz.sequential.inferno.500": Color ->
        primitive::COLOR_ORANGE_500;
    /// Sequential inferno scale, step 600.
    DATA_VIZ_SEQUENTIAL_INFERNO_600 = "data-viz.sequential.inferno.600": Color ->
        primitive::COLOR_ORANGE_600;
    /// Sequential inferno scale, step 700.
    DATA_VIZ_SEQUENTIAL_INFERNO_700 = "data-viz.sequential.inferno.700": Color ->
        primitive::COLOR_ORANGE_700;
    /// Sequential inferno scale, step 800.
    DATA_VIZ_SEQUENTIAL_INFERNO_800 = "data-viz.sequential.inferno.800": Color ->
        primitive::COLOR_ORANGE_800;
    /// Sequential inferno scale, step 900.
    DATA_VIZ_SEQUENTIAL_INFERNO_900 = "data-viz.sequential.inferno.900": Color ->
        primitive::COLOR_ORANGE_900;
    /// Sequential inferno scale, step 1000.
    DATA_VIZ_SEQUENTIAL_INFERNO_1000 = "data-viz.sequential.inferno.1000": Color ->
        primitive::COLOR_ORANGE_1000;
    /// Sequential inferno scale, step 1100.
    DATA_VIZ_SEQUENTIAL_INFERNO_1100 = "data-viz.sequential.inferno.1100": Color ->
        primitive::COLOR_ORANGE_1100;
    /// Sequential inferno scale, step 1200.
    DATA_VIZ_SEQUENTIAL_INFERNO_1200 = "data-viz.sequential.inferno.1200": Color ->
        primitive::COLOR_ORANGE_1200;
    /// Sequential inferno scale, step 1300.
    DATA_VIZ_SEQUENTIAL_INFERNO_1300 = "data-viz.sequential.inferno.1300": Color ->
        primitive::COLOR_ORANGE_1300;
    /// Sequential inferno scale, step 1400.
    DATA_VIZ_SEQUENTIAL_INFERNO_1400 = "data-viz.sequential.inferno.1400": Color ->
        primitive::COLOR_ORANGE_1400;
    /// Sequential inferno scale, step 1500.
    DATA_VIZ_SEQUENTIAL_INFERNO_1500 = "data-viz.sequential.inferno.1500": Color ->
        primitive::COLOR_ORANGE_1500;
    /// Sequential inferno scale, step 1600.
    DATA_VIZ_SEQUENTIAL_INFERNO_1600 = "data-viz.sequential.inferno.1600": Color ->
        primitive::COLOR_ORANGE_1600;

    /// Sequential cividis scale, step 100.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_100 = "data-viz.sequential.cividis.100": Color ->
        primitive::COLOR_YELLOW_100;
    /// Sequential cividis scale, step 200.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_200 = "data-viz.sequential.cividis.200": Color ->
        primitive::COLOR_YELLOW_200;
    /// Sequential cividis scale, step 300.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_300 = "data-viz.sequential.cividis.300": Color ->
        primitive::COLOR_YELLOW_300;
    /// Sequential cividis scale, step 400.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_400 = "data-viz.sequential.cividis.400": Color ->
        primitive::COLOR_YELLOW_400;
    /// Sequential cividis scale, step 500.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_500 = "data-viz.sequential.cividis.500": Color ->
        primitive::COLOR_YELLOW_500;
    /// Sequential cividis scale, step 600.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_600 = "data-viz.sequential.cividis.600": Color ->
        primitive::COLOR_YELLOW_600;
    /// Sequential cividis scale, step 700.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_700 = "data-viz.sequential.cividis.700": Color ->
        primitive::COLOR_YELLOW_700;
    /// Sequential cividis scale, step 800.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_800 = "data-viz.sequential.cividis.800": Color ->
        primitive::COLOR_YELLOW_800;
    /// Sequential cividis scale, step 900.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_900 = "data-viz.sequential.cividis.900": Color ->
        primitive::COLOR_YELLOW_900;
    /// Sequential cividis scale, step 1000.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_1000 = "data-viz.sequential.cividis.1000": Color ->
        primitive::COLOR_YELLOW_1000;
    /// Sequential cividis scale, step 1100.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_1100 = "data-viz.sequential.cividis.1100": Color ->
        primitive::COLOR_YELLOW_1100;
    /// Sequential cividis scale, step 1200.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_1200 = "data-viz.sequential.cividis.1200": Color ->
        primitive::COLOR_YELLOW_1200;
    /// Sequential cividis scale, step 1300.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_1300 = "data-viz.sequential.cividis.1300": Color ->
        primitive::COLOR_YELLOW_1300;
    /// Sequential cividis scale, step 1400.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_1400 = "data-viz.sequential.cividis.1400": Color ->
        primitive::COLOR_YELLOW_1400;
    /// Sequential cividis scale, step 1500.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_1500 = "data-viz.sequential.cividis.1500": Color ->
        primitive::COLOR_YELLOW_1500;
    /// Sequential cividis scale, step 1600.
    DATA_VIZ_SEQUENTIAL_CIVIDIS_1600 = "data-viz.sequential.cividis.1600": Color ->
        primitive::COLOR_YELLOW_1600;

    /// Diverging red to blue scale, step 100.
    DATA_VIZ_DIVERGING_RD_BU_100 = "data-viz.diverging.rd-bu.100": Color ->
        primitive::COLOR_RED_1600;
    /// Diverging red to blue scale, step 200.
    DATA_VIZ_DIVERGING_RD_BU_200 = "data-viz.diverging.rd-bu.200": Color ->
        primitive::COLOR_RED_1500;
    /// Diverging red to blue scale, step 300.
    DATA_VIZ_DIVERGING_RD_BU_300 = "data-viz.diverging.rd-bu.300": Color ->
        primitive::COLOR_RED_1400;
    /// Diverging red to blue scale, step 400.
    DATA_VIZ_DIVERGING_RD_BU_400 = "data-viz.diverging.rd-bu.400": Color ->
        primitive::COLOR_RED_1300;
    /// Diverging red to blue scale, step 500.
    DATA_VIZ_DIVERGING_RD_BU_500 = "data-viz.diverging.rd-bu.500": Color ->
        primitive::COLOR_RED_1200;
    /// Diverging red to blue scale, step 600.
    DATA_VIZ_DIVERGING_RD_BU_600 = "data-viz.diverging.rd-bu.600": Color ->
        primitive::COLOR_RED_1100;
    /// Diverging red to blue scale, step 700.
    DATA_VIZ_DIVERGING_RD_BU_700 = "data-viz.diverging.rd-bu.700": Color ->
        primitive::COLOR_RED_1000;
    /// Diverging red to blue scale, step 800.
    DATA_VIZ_DIVERGING_RD_BU_800 = "data-viz.diverging.rd-bu.800": Color ->
        primitive::COLOR_RED_900;
    /// Diverging red to blue scale, step 900.
    DATA_VIZ_DIVERGING_RD_BU_900 = "data-viz.diverging.rd-bu.900": Color ->
        primitive::COLOR_BLUE_900;
    /// Diverging red to blue scale, step 1000.
    DATA_VIZ_DIVERGING_RD_BU_1000 = "data-viz.diverging.rd-bu.1000": Color ->
        primitive::COLOR_BLUE_1000;
    /// Diverging red to blue scale, step 1100.
    DATA_VIZ_DIVERGING_RD_BU_1100 = "data-viz.diverging.rd-bu.1100": Color ->
        primitive::COLOR_BLUE_1100;
    /// Diverging red to blue scale, step 1200.
    DATA_VIZ_DIVERGING_RD_BU_1200 = "data-viz.diverging.rd-bu.1200": Color ->
        primitive::COLOR_BLUE_1200;
    /// Diverging red to blue scale, step 1300.
    DATA_VIZ_DIVERGING_RD_BU_1300 = "data-viz.diverging.rd-bu.1300": Color ->
        primitive::COLOR_BLUE_1300;
    /// Diverging red to blue scale, step 1400.
    DATA_VIZ_DIVERGING_RD_BU_1400 = "data-viz.diverging.rd-bu.1400": Color ->
        primitive::COLOR_BLUE_1400;
    /// Diverging red to blue scale, step 1500.
    DATA_VIZ_DIVERGING_RD_BU_1500 = "data-viz.diverging.rd-bu.1500": Color ->
        primitive::COLOR_BLUE_1500;
    /// Diverging red to blue scale, step 1600.
    DATA_VIZ_DIVERGING_RD_BU_1600 = "data-viz.diverging.rd-bu.1600": Color ->
        primitive::COLOR_BLUE_1600;

    /// Diverging purple to green scale, step 100.
    DATA_VIZ_DIVERGING_PU_GN_100 = "data-viz.diverging.pu-gn.100": Color ->
        primitive::COLOR_PURPLE_1600;
    /// Diverging purple to green scale, step 200.
    DATA_VIZ_DIVERGING_PU_GN_200 = "data-viz.diverging.pu-gn.200": Color ->
        primitive::COLOR_PURPLE_1500;
    /// Diverging purple to green scale, step 300.
    DATA_VIZ_DIVERGING_PU_GN_300 = "data-viz.diverging.pu-gn.300": Color ->
        primitive::COLOR_PURPLE_1400;
    /// Diverging purple to green scale, step 400.
    DATA_VIZ_DIVERGING_PU_GN_400 = "data-viz.diverging.pu-gn.400": Color ->
        primitive::COLOR_PURPLE_1300;
    /// Diverging purple to green scale, step 500.
    DATA_VIZ_DIVERGING_PU_GN_500 = "data-viz.diverging.pu-gn.500": Color ->
        primitive::COLOR_PURPLE_1200;
    /// Diverging purple to green scale, step 600.
    DATA_VIZ_DIVERGING_PU_GN_600 = "data-viz.diverging.pu-gn.600": Color ->
        primitive::COLOR_PURPLE_1100;
    /// Diverging purple to green scale, step 700.
    DATA_VIZ_DIVERGING_PU_GN_700 = "data-viz.diverging.pu-gn.700": Color ->
        primitive::COLOR_PURPLE_1000;
    /// Diverging purple to green scale, step 800.
    DATA_VIZ_DIVERGING_PU_GN_800 = "data-viz.diverging.pu-gn.800": Color ->
        primitive::COLOR_PURPLE_900;
    /// Diverging purple to green scale, step 900.
    DATA_VIZ_DIVERGING_PU_GN_900 = "data-viz.diverging.pu-gn.900": Color ->
        primitive::COLOR_GREEN_900;
    /// Diverging purple to green scale, step 1000.
    DATA_VIZ_DIVERGING_PU_GN_1000 = "data-viz.diverging.pu-gn.1000": Color ->
        primitive::COLOR_GREEN_1000;
    /// Diverging purple to green scale, step 1100.
    DATA_VIZ_DIVERGING_PU_GN_1100 = "data-viz.diverging.pu-gn.1100": Color ->
        primitive::COLOR_GREEN_1100;
    /// Diverging purple to green scale, step 1200.
    DATA_VIZ_DIVERGING_PU_GN_1200 = "data-viz.diverging.pu-gn.1200": Color ->
        primitive::COLOR_GREEN_1200;
    /// Diverging purple to green scale, step 1300.
    DATA_VIZ_DIVERGING_PU_GN_1300 = "data-viz.diverging.pu-gn.1300": Color ->
        primitive::COLOR_GREEN_1300;
    /// Diverging purple to green scale, step 1400.
    DATA_VIZ_DIVERGING_PU_GN_1400 = "data-viz.diverging.pu-gn.1400": Color ->
        primitive::COLOR_GREEN_1400;
    /// Diverging purple to green scale, step 1500.
    DATA_VIZ_DIVERGING_PU_GN_1500 = "data-viz.diverging.pu-gn.1500": Color ->
        primitive::COLOR_GREEN_1500;
    /// Diverging purple to green scale, step 1600.
    DATA_VIZ_DIVERGING_PU_GN_1600 = "data-viz.diverging.pu-gn.1600": Color ->
        primitive::COLOR_GREEN_1600;

    /// Diverging brown to teal scale, step 100.
    DATA_VIZ_DIVERGING_BR_TEAL_100 = "data-viz.diverging.br-teal.100": Color ->
        primitive::COLOR_BROWN_1600;
    /// Diverging brown to teal scale, step 200.
    DATA_VIZ_DIVERGING_BR_TEAL_200 = "data-viz.diverging.br-teal.200": Color ->
        primitive::COLOR_BROWN_1500;
    /// Diverging brown to teal scale, step 300.
    DATA_VIZ_DIVERGING_BR_TEAL_300 = "data-viz.diverging.br-teal.300": Color ->
        primitive::COLOR_BROWN_1400;
    /// Diverging brown to teal scale, step 400.
    DATA_VIZ_DIVERGING_BR_TEAL_400 = "data-viz.diverging.br-teal.400": Color ->
        primitive::COLOR_BROWN_1300;
    /// Diverging brown to teal scale, step 500.
    DATA_VIZ_DIVERGING_BR_TEAL_500 = "data-viz.diverging.br-teal.500": Color ->
        primitive::COLOR_BROWN_1200;
    /// Diverging brown to teal scale, step 600.
    DATA_VIZ_DIVERGING_BR_TEAL_600 = "data-viz.diverging.br-teal.600": Color ->
        primitive::COLOR_BROWN_1100;
    /// Diverging brown to teal scale, step 700.
    DATA_VIZ_DIVERGING_BR_TEAL_700 = "data-viz.diverging.br-teal.700": Color ->
        primitive::COLOR_BROWN_1000;
    /// Diverging brown to teal scale, step 800.
    DATA_VIZ_DIVERGING_BR_TEAL_800 = "data-viz.diverging.br-teal.800": Color ->
        primitive::COLOR_BROWN_900;
    /// Diverging brown to teal scale, step 900.
    DATA_VIZ_DIVERGING_BR_TEAL_900 = "data-viz.diverging.br-teal.900": Color ->
        primitive::COLOR_TURQUOISE_900;
    /// Diverging brown to teal scale, step 1000.
    DATA_VIZ_DIVERGING_BR_TEAL_1000 = "data-viz.diverging.br-teal.1000": Color ->
        primitive::COLOR_TURQUOISE_1000;
    /// Diverging brown to teal scale, step 1100.
    DATA_VIZ_DIVERGING_BR_TEAL_1100 = "data-viz.diverging.br-teal.1100": Color ->
        primitive::COLOR_TURQUOISE_1100;
    /// Diverging brown to teal scale, step 1200.
    DATA_VIZ_DIVERGING_BR_TEAL_1200 = "data-viz.diverging.br-teal.1200": Color ->
        primitive::COLOR_TURQUOISE_1200;
    /// Diverging brown to teal scale, step 1300.
    DATA_VIZ_DIVERGING_BR_TEAL_1300 = "data-viz.diverging.br-teal.1300": Color ->
        primitive::COLOR_TURQUOISE_1300;
    /// Diverging brown to teal scale, step 1400.
    DATA_VIZ_DIVERGING_BR_TEAL_1400 = "data-viz.diverging.br-teal.1400": Color ->
        primitive::COLOR_TURQUOISE_1400;
    /// Diverging brown to teal scale, step 1500.
    DATA_VIZ_DIVERGING_BR_TEAL_1500 = "data-viz.diverging.br-teal.1500": Color ->
        primitive::COLOR_TURQUOISE_1500;
    /// Diverging brown to teal scale, step 1600.
    DATA_VIZ_DIVERGING_BR_TEAL_1600 = "data-viz.diverging.br-teal.1600": Color ->
        primitive::COLOR_TURQUOISE_1600;

    /// Color of chart axes.
    DATA_VIZ_AXIS_DEFAULT = "data-viz.axis.default": Color -> primitive::COLOR_GRAY_400;

    /// Color of chart grid lines.
    DATA_VIZ_GRID_DEFAULT = "data-viz.grid.default": Color -> primitive::COLOR_GRAY_200;

    /// Color of chart labels.
    DATA_VIZ_LABEL_DEFAULT = "data-viz.label.default": Color -> primitive::COLOR_GRAY_700;

    /// Highlight of the hovered or selected data point.
    DATA_VIZ_HIGHLIGHT_DEFAULT = "data-viz.highlight.default": Color -> ACCENT_COLOR_900;
}

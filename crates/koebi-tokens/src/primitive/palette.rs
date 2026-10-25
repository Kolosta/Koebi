//! Hue palettes: 18 hues in 16 steps, from 100 (lightest) to 1600 (darkest).

use koebi_design::define::*;

koebi_design::tokens! {
    group PALETTE: Primitive, start = 0;

    /// Red, palette step 100.
    COLOR_RED_100 = "color.red.100": Color = hex("#FFF6F5");
    /// Red, palette step 200.
    COLOR_RED_200 = "color.red.200": Color = hex("#FFEBE8");
    /// Red, palette step 300.
    COLOR_RED_300 = "color.red.300": Color = hex("#FFD6D1");
    /// Red, palette step 400.
    COLOR_RED_400 = "color.red.400": Color = hex("#FFBCB4");
    /// Red, palette step 500.
    COLOR_RED_500 = "color.red.500": Color = hex("#FF9D91");
    /// Red, palette step 600.
    COLOR_RED_600 = "color.red.600": Color = hex("#FF7665");
    /// Red, palette step 700.
    COLOR_RED_700 = "color.red.700": Color = hex("#FF513D");
    /// Red, palette step 800.
    COLOR_RED_800 = "color.red.800": Color = hex("#F03823");
    /// Red, palette step 900.
    COLOR_RED_900 = "color.red.900": Color = hex("#D73220");
    /// Red, palette step 1000.
    COLOR_RED_1000 = "color.red.1000": Color = hex("#B72818");
    /// Red, palette step 1100.
    COLOR_RED_1100 = "color.red.1100": Color = hex("#9C2113");
    /// Red, palette step 1200.
    COLOR_RED_1200 = "color.red.1200": Color = hex("#811B0E");
    /// Red, palette step 1300.
    COLOR_RED_1300 = "color.red.1300": Color = hex("#68150A");
    /// Red, palette step 1400.
    COLOR_RED_1400 = "color.red.1400": Color = hex("#501006");
    /// Red, palette step 1500.
    COLOR_RED_1500 = "color.red.1500": Color = hex("#3B0B04");
    /// Red, palette step 1600.
    COLOR_RED_1600 = "color.red.1600": Color = hex("#1D0502");

    /// Orange, palette step 100.
    COLOR_ORANGE_100 = "color.orange.100": Color = hex("#FFF6E7");
    /// Orange, palette step 200.
    COLOR_ORANGE_200 = "color.orange.200": Color = hex("#FFECCF");
    /// Orange, palette step 300.
    COLOR_ORANGE_300 = "color.orange.300": Color = hex("#FFDA9E");
    /// Orange, palette step 400.
    COLOR_ORANGE_400 = "color.orange.400": Color = hex("#FFC15E");
    /// Orange, palette step 500.
    COLOR_ORANGE_500 = "color.orange.500": Color = hex("#FFA213");
    /// Orange, palette step 600.
    COLOR_ORANGE_600 = "color.orange.600": Color = hex("#FC7D00");
    /// Orange, palette step 700.
    COLOR_ORANGE_700 = "color.orange.700": Color = hex("#E86A00");
    /// Orange, palette step 800.
    COLOR_ORANGE_800 = "color.orange.800": Color = hex("#D45B00");
    /// Orange, palette step 900.
    COLOR_ORANGE_900 = "color.orange.900": Color = hex("#C24E00");
    /// Orange, palette step 1000.
    COLOR_ORANGE_1000 = "color.orange.1000": Color = hex("#A73E00");
    /// Orange, palette step 1100.
    COLOR_ORANGE_1100 = "color.orange.1100": Color = hex("#903300");
    /// Orange, palette step 1200.
    COLOR_ORANGE_1200 = "color.orange.1200": Color = hex("#762900");
    /// Orange, palette step 1300.
    COLOR_ORANGE_1300 = "color.orange.1300": Color = hex("#5F2000");
    /// Orange, palette step 1400.
    COLOR_ORANGE_1400 = "color.orange.1400": Color = hex("#491800");
    /// Orange, palette step 1500.
    COLOR_ORANGE_1500 = "color.orange.1500": Color = hex("#341200");
    /// Orange, palette step 1600.
    COLOR_ORANGE_1600 = "color.orange.1600": Color = hex("#190800");

    /// Yellow, palette step 100.
    COLOR_YELLOW_100 = "color.yellow.100": Color = hex("#FFF8CC");
    /// Yellow, palette step 200.
    COLOR_YELLOW_200 = "color.yellow.200": Color = hex("#FFF197");
    /// Yellow, palette step 300.
    COLOR_YELLOW_300 = "color.yellow.300": Color = hex("#FFDE2C");
    /// Yellow, palette step 400.
    COLOR_YELLOW_400 = "color.yellow.400": Color = hex("#F5C700");
    /// Yellow, palette step 500.
    COLOR_YELLOW_500 = "color.yellow.500": Color = hex("#E6AF00");
    /// Yellow, palette step 600.
    COLOR_YELLOW_600 = "color.yellow.600": Color = hex("#D29500");
    /// Yellow, palette step 700.
    COLOR_YELLOW_700 = "color.yellow.700": Color = hex("#C18300");
    /// Yellow, palette step 800.
    COLOR_YELLOW_800 = "color.yellow.800": Color = hex("#AF7400");
    /// Yellow, palette step 900.
    COLOR_YELLOW_900 = "color.yellow.900": Color = hex("#9E6600");
    /// Yellow, palette step 1000.
    COLOR_YELLOW_1000 = "color.yellow.1000": Color = hex("#865500");
    /// Yellow, palette step 1100.
    COLOR_YELLOW_1100 = "color.yellow.1100": Color = hex("#724800");
    /// Yellow, palette step 1200.
    COLOR_YELLOW_1200 = "color.yellow.1200": Color = hex("#5D3B00");
    /// Yellow, palette step 1300.
    COLOR_YELLOW_1300 = "color.yellow.1300": Color = hex("#4B2F00");
    /// Yellow, palette step 1400.
    COLOR_YELLOW_1400 = "color.yellow.1400": Color = hex("#382300");
    /// Yellow, palette step 1500.
    COLOR_YELLOW_1500 = "color.yellow.1500": Color = hex("#281900");
    /// Yellow, palette step 1600.
    COLOR_YELLOW_1600 = "color.yellow.1600": Color = hex("#120B00");

    /// Chartreuse, palette step 100.
    COLOR_CHARTREUSE_100 = "color.chartreuse.100": Color = hex("#F6FBDE");
    /// Chartreuse, palette step 200.
    COLOR_CHARTREUSE_200 = "color.chartreuse.200": Color = hex("#EAF6AD");
    /// Chartreuse, palette step 300.
    COLOR_CHARTREUSE_300 = "color.chartreuse.300": Color = hex("#D0EC46");
    /// Chartreuse, palette step 400.
    COLOR_CHARTREUSE_400 = "color.chartreuse.400": Color = hex("#B6DB00");
    /// Chartreuse, palette step 500.
    COLOR_CHARTREUSE_500 = "color.chartreuse.500": Color = hex("#A3C400");
    /// Chartreuse, palette step 600.
    COLOR_CHARTREUSE_600 = "color.chartreuse.600": Color = hex("#8FAC00");
    /// Chartreuse, palette step 700.
    COLOR_CHARTREUSE_700 = "color.chartreuse.700": Color = hex("#809900");
    /// Chartreuse, palette step 800.
    COLOR_CHARTREUSE_800 = "color.chartreuse.800": Color = hex("#728900");
    /// Chartreuse, palette step 900.
    COLOR_CHARTREUSE_900 = "color.chartreuse.900": Color = hex("#667A00");
    /// Chartreuse, palette step 1000.
    COLOR_CHARTREUSE_1000 = "color.chartreuse.1000": Color = hex("#566700");
    /// Chartreuse, palette step 1100.
    COLOR_CHARTREUSE_1100 = "color.chartreuse.1100": Color = hex("#495700");
    /// Chartreuse, palette step 1200.
    COLOR_CHARTREUSE_1200 = "color.chartreuse.1200": Color = hex("#3C4700");
    /// Chartreuse, palette step 1300.
    COLOR_CHARTREUSE_1300 = "color.chartreuse.1300": Color = hex("#2F3900");
    /// Chartreuse, palette step 1400.
    COLOR_CHARTREUSE_1400 = "color.chartreuse.1400": Color = hex("#232B00");
    /// Chartreuse, palette step 1500.
    COLOR_CHARTREUSE_1500 = "color.chartreuse.1500": Color = hex("#191E00");
    /// Chartreuse, palette step 1600.
    COLOR_CHARTREUSE_1600 = "color.chartreuse.1600": Color = hex("#0B0E00");

    /// Celery, palette step 100.
    COLOR_CELERY_100 = "color.celery.100": Color = hex("#EBFFDC");
    /// Celery, palette step 200.
    COLOR_CELERY_200 = "color.celery.200": Color = hex("#C5FF9C");
    /// Celery, palette step 300.
    COLOR_CELERY_300 = "color.celery.300": Color = hex("#9DF75C");
    /// Celery, palette step 400.
    COLOR_CELERY_400 = "color.celery.400": Color = hex("#81E43A");
    /// Celery, palette step 500.
    COLOR_CELERY_500 = "color.celery.500": Color = hex("#6ECE2A");
    /// Celery, palette step 600.
    COLOR_CELERY_600 = "color.celery.600": Color = hex("#5DB41F");
    /// Celery, palette step 700.
    COLOR_CELERY_700 = "color.celery.700": Color = hex("#52A119");
    /// Celery, palette step 800.
    COLOR_CELERY_800 = "color.celery.800": Color = hex("#489014");
    /// Celery, palette step 900.
    COLOR_CELERY_900 = "color.celery.900": Color = hex("#408111");
    /// Celery, palette step 1000.
    COLOR_CELERY_1000 = "color.celery.1000": Color = hex("#346D0C");
    /// Celery, palette step 1100.
    COLOR_CELERY_1100 = "color.celery.1100": Color = hex("#2C5C09");
    /// Celery, palette step 1200.
    COLOR_CELERY_1200 = "color.celery.1200": Color = hex("#234B06");
    /// Celery, palette step 1300.
    COLOR_CELERY_1300 = "color.celery.1300": Color = hex("#1B3C03");
    /// Celery, palette step 1400.
    COLOR_CELERY_1400 = "color.celery.1400": Color = hex("#132E00");
    /// Celery, palette step 1500.
    COLOR_CELERY_1500 = "color.celery.1500": Color = hex("#0C2100");
    /// Celery, palette step 1600.
    COLOR_CELERY_1600 = "color.celery.1600": Color = hex("#040F00");

    /// Green, palette step 100.
    COLOR_GREEN_100 = "color.green.100": Color = hex("#EDFCF1");
    /// Green, palette step 200.
    COLOR_GREEN_200 = "color.green.200": Color = hex("#D7F7E1");
    /// Green, palette step 300.
    COLOR_GREEN_300 = "color.green.300": Color = hex("#ADEEC5");
    /// Green, palette step 400.
    COLOR_GREEN_400 = "color.green.400": Color = hex("#6BE3A2");
    /// Green, palette step 500.
    COLOR_GREEN_500 = "color.green.500": Color = hex("#2BD17D");
    /// Green, palette step 600.
    COLOR_GREEN_600 = "color.green.600": Color = hex("#12B867");
    /// Green, palette step 700.
    COLOR_GREEN_700 = "color.green.700": Color = hex("#0BA45D");
    /// Green, palette step 800.
    COLOR_GREEN_800 = "color.green.800": Color = hex("#079355");
    /// Green, palette step 900.
    COLOR_GREEN_900 = "color.green.900": Color = hex("#05834E");
    /// Green, palette step 1000.
    COLOR_GREEN_1000 = "color.green.1000": Color = hex("#036E45");
    /// Green, palette step 1100.
    COLOR_GREEN_1100 = "color.green.1100": Color = hex("#025D3C");
    /// Green, palette step 1200.
    COLOR_GREEN_1200 = "color.green.1200": Color = hex("#014C34");
    /// Green, palette step 1300.
    COLOR_GREEN_1300 = "color.green.1300": Color = hex("#003D2C");
    /// Green, palette step 1400.
    COLOR_GREEN_1400 = "color.green.1400": Color = hex("#002E22");
    /// Green, palette step 1500.
    COLOR_GREEN_1500 = "color.green.1500": Color = hex("#002119");
    /// Green, palette step 1600.
    COLOR_GREEN_1600 = "color.green.1600": Color = hex("#000F0C");

    /// Seafoam, palette step 100.
    COLOR_SEAFOAM_100 = "color.seafoam.100": Color = hex("#EBFBF6");
    /// Seafoam, palette step 200.
    COLOR_SEAFOAM_200 = "color.seafoam.200": Color = hex("#D3F6EA");
    /// Seafoam, palette step 300.
    COLOR_SEAFOAM_300 = "color.seafoam.300": Color = hex("#A9EDD8");
    /// Seafoam, palette step 400.
    COLOR_SEAFOAM_400 = "color.seafoam.400": Color = hex("#5CE1C2");
    /// Seafoam, palette step 500.
    COLOR_SEAFOAM_500 = "color.seafoam.500": Color = hex("#10CFA9");
    /// Seafoam, palette step 600.
    COLOR_SEAFOAM_600 = "color.seafoam.600": Color = hex("#0DB595");
    /// Seafoam, palette step 700.
    COLOR_SEAFOAM_700 = "color.seafoam.700": Color = hex("#0BA286");
    /// Seafoam, palette step 800.
    COLOR_SEAFOAM_800 = "color.seafoam.800": Color = hex("#099078");
    /// Seafoam, palette step 900.
    COLOR_SEAFOAM_900 = "color.seafoam.900": Color = hex("#07816D");
    /// Seafoam, palette step 1000.
    COLOR_SEAFOAM_1000 = "color.seafoam.1000": Color = hex("#056C5C");
    /// Seafoam, palette step 1100.
    COLOR_SEAFOAM_1100 = "color.seafoam.1100": Color = hex("#035C50");
    /// Seafoam, palette step 1200.
    COLOR_SEAFOAM_1200 = "color.seafoam.1200": Color = hex("#014B43");
    /// Seafoam, palette step 1300.
    COLOR_SEAFOAM_1300 = "color.seafoam.1300": Color = hex("#003C36");
    /// Seafoam, palette step 1400.
    COLOR_SEAFOAM_1400 = "color.seafoam.1400": Color = hex("#002E28");
    /// Seafoam, palette step 1500.
    COLOR_SEAFOAM_1500 = "color.seafoam.1500": Color = hex("#00211D");
    /// Seafoam, palette step 1600.
    COLOR_SEAFOAM_1600 = "color.seafoam.1600": Color = hex("#000F0E");

    /// Cyan, palette step 100.
    COLOR_CYAN_100 = "color.cyan.100": Color = hex("#EEFAFE");
    /// Cyan, palette step 200.
    COLOR_CYAN_200 = "color.cyan.200": Color = hex("#D9F4FD");
    /// Cyan, palette step 300.
    COLOR_CYAN_300 = "color.cyan.300": Color = hex("#B7E7FC");
    /// Cyan, palette step 400.
    COLOR_CYAN_400 = "color.cyan.400": Color = hex("#8AD5FF");
    /// Cyan, palette step 500.
    COLOR_CYAN_500 = "color.cyan.500": Color = hex("#5CC0FF");
    /// Cyan, palette step 600.
    COLOR_CYAN_600 = "color.cyan.600": Color = hex("#30A7FE");
    /// Cyan, palette step 700.
    COLOR_CYAN_700 = "color.cyan.700": Color = hex("#1D95E7");
    /// Cyan, palette step 800.
    COLOR_CYAN_800 = "color.cyan.800": Color = hex("#1286CD");
    /// Cyan, palette step 900.
    COLOR_CYAN_900 = "color.cyan.900": Color = hex("#0B78B3");
    /// Cyan, palette step 1000.
    COLOR_CYAN_1000 = "color.cyan.1000": Color = hex("#046691");
    /// Cyan, palette step 1100.
    COLOR_CYAN_1100 = "color.cyan.1100": Color = hex("#005779");
    /// Cyan, palette step 1200.
    COLOR_CYAN_1200 = "color.cyan.1200": Color = hex("#004762");
    /// Cyan, palette step 1300.
    COLOR_CYAN_1300 = "color.cyan.1300": Color = hex("#00394E");
    /// Cyan, palette step 1400.
    COLOR_CYAN_1400 = "color.cyan.1400": Color = hex("#002B3B");
    /// Cyan, palette step 1500.
    COLOR_CYAN_1500 = "color.cyan.1500": Color = hex("#001F2B");
    /// Cyan, palette step 1600.
    COLOR_CYAN_1600 = "color.cyan.1600": Color = hex("#000E14");

    /// Blue, palette step 100.
    COLOR_BLUE_100 = "color.blue.100": Color = hex("#F5F9FF");
    /// Blue, palette step 200.
    COLOR_BLUE_200 = "color.blue.200": Color = hex("#E5F0FE");
    /// Blue, palette step 300.
    COLOR_BLUE_300 = "color.blue.300": Color = hex("#CBE2FE");
    /// Blue, palette step 400.
    COLOR_BLUE_400 = "color.blue.400": Color = hex("#ACCFFD");
    /// Blue, palette step 500.
    COLOR_BLUE_500 = "color.blue.500": Color = hex("#8EB9FC");
    /// Blue, palette step 600.
    COLOR_BLUE_600 = "color.blue.600": Color = hex("#729EFD");
    /// Blue, palette step 700.
    COLOR_BLUE_700 = "color.blue.700": Color = hex("#5D89FF");
    /// Blue, palette step 800.
    COLOR_BLUE_800 = "color.blue.800": Color = hex("#4B75FF");
    /// Blue, palette step 900.
    COLOR_BLUE_900 = "color.blue.900": Color = hex("#3B63FB");
    /// Blue, palette step 1000.
    COLOR_BLUE_1000 = "color.blue.1000": Color = hex("#274DEA");
    /// Blue, palette step 1100.
    COLOR_BLUE_1100 = "color.blue.1100": Color = hex("#1D3ECF");
    /// Blue, palette step 1200.
    COLOR_BLUE_1200 = "color.blue.1200": Color = hex("#1532AD");
    /// Blue, palette step 1300.
    COLOR_BLUE_1300 = "color.blue.1300": Color = hex("#10288C");
    /// Blue, palette step 1400.
    COLOR_BLUE_1400 = "color.blue.1400": Color = hex("#0C1F69");
    /// Blue, palette step 1500.
    COLOR_BLUE_1500 = "color.blue.1500": Color = hex("#0E1843");
    /// Blue, palette step 1600.
    COLOR_BLUE_1600 = "color.blue.1600": Color = hex("#070B1E");

    /// Indigo, palette step 100.
    COLOR_INDIGO_100 = "color.indigo.100": Color = hex("#F7F8FF");
    /// Indigo, palette step 200.
    COLOR_INDIGO_200 = "color.indigo.200": Color = hex("#EBEEFF");
    /// Indigo, palette step 300.
    COLOR_INDIGO_300 = "color.indigo.300": Color = hex("#D8DEFF");
    /// Indigo, palette step 400.
    COLOR_INDIGO_400 = "color.indigo.400": Color = hex("#C0C9FF");
    /// Indigo, palette step 500.
    COLOR_INDIGO_500 = "color.indigo.500": Color = hex("#A7B2FF");
    /// Indigo, palette step 600.
    COLOR_INDIGO_600 = "color.indigo.600": Color = hex("#9197FE");
    /// Indigo, palette step 700.
    COLOR_INDIGO_700 = "color.indigo.700": Color = hex("#8480FE");
    /// Indigo, palette step 800.
    COLOR_INDIGO_800 = "color.indigo.800": Color = hex("#7A6AFD");
    /// Indigo, palette step 900.
    COLOR_INDIGO_900 = "color.indigo.900": Color = hex("#7155FA");
    /// Indigo, palette step 1000.
    COLOR_INDIGO_1000 = "color.indigo.1000": Color = hex("#6338EE");
    /// Indigo, palette step 1100.
    COLOR_INDIGO_1100 = "color.indigo.1100": Color = hex("#5424DB");
    /// Indigo, palette step 1200.
    COLOR_INDIGO_1200 = "color.indigo.1200": Color = hex("#4513BF");
    /// Indigo, palette step 1300.
    COLOR_INDIGO_1300 = "color.indigo.1300": Color = hex("#3706A0");
    /// Indigo, palette step 1400.
    COLOR_INDIGO_1400 = "color.indigo.1400": Color = hex("#2A0081");
    /// Indigo, palette step 1500.
    COLOR_INDIGO_1500 = "color.indigo.1500": Color = hex("#1F0062");
    /// Indigo, palette step 1600.
    COLOR_INDIGO_1600 = "color.indigo.1600": Color = hex("#110036");

    /// Purple, palette step 100.
    COLOR_PURPLE_100 = "color.purple.100": Color = hex("#FBF7FE");
    /// Purple, palette step 200.
    COLOR_PURPLE_200 = "color.purple.200": Color = hex("#F4EBFC");
    /// Purple, palette step 300.
    COLOR_PURPLE_300 = "color.purple.300": Color = hex("#EBDAF9");
    /// Purple, palette step 400.
    COLOR_PURPLE_400 = "color.purple.400": Color = hex("#DDC1F6");
    /// Purple, palette step 500.
    COLOR_PURPLE_500 = "color.purple.500": Color = hex("#D0A7F3");
    /// Purple, palette step 600.
    COLOR_PURPLE_600 = "color.purple.600": Color = hex("#BF8AEE");
    /// Purple, palette step 700.
    COLOR_PURPLE_700 = "color.purple.700": Color = hex("#B272EB");
    /// Purple, palette step 800.
    COLOR_PURPLE_800 = "color.purple.800": Color = hex("#A65CE7");
    /// Purple, palette step 900.
    COLOR_PURPLE_900 = "color.purple.900": Color = hex("#9A47E2");
    /// Purple, palette step 1000.
    COLOR_PURPLE_1000 = "color.purple.1000": Color = hex("#8628D9");
    /// Purple, palette step 1100.
    COLOR_PURPLE_1100 = "color.purple.1100": Color = hex("#730DCC");
    /// Purple, palette step 1200.
    COLOR_PURPLE_1200 = "color.purple.1200": Color = hex("#5D00B1");
    /// Purple, palette step 1300.
    COLOR_PURPLE_1300 = "color.purple.1300": Color = hex("#4B0090");
    /// Purple, palette step 1400.
    COLOR_PURPLE_1400 = "color.purple.1400": Color = hex("#3B006F");
    /// Purple, palette step 1500.
    COLOR_PURPLE_1500 = "color.purple.1500": Color = hex("#2C0054");
    /// Purple, palette step 1600.
    COLOR_PURPLE_1600 = "color.purple.1600": Color = hex("#17002D");

    /// Fuchsia, palette step 100.
    COLOR_FUCHSIA_100 = "color.fuchsia.100": Color = hex("#FEF6FF");
    /// Fuchsia, palette step 200.
    COLOR_FUCHSIA_200 = "color.fuchsia.200": Color = hex("#FDE9FF");
    /// Fuchsia, palette step 300.
    COLOR_FUCHSIA_300 = "color.fuchsia.300": Color = hex("#FAD3FF");
    /// Fuchsia, palette step 400.
    COLOR_FUCHSIA_400 = "color.fuchsia.400": Color = hex("#F7B5FF");
    /// Fuchsia, palette step 500.
    COLOR_FUCHSIA_500 = "color.fuchsia.500": Color = hex("#F393FF");
    /// Fuchsia, palette step 600.
    COLOR_FUCHSIA_600 = "color.fuchsia.600": Color = hex("#EC69FF");
    /// Fuchsia, palette step 700.
    COLOR_FUCHSIA_700 = "color.fuchsia.700": Color = hex("#DF4DF5");
    /// Fuchsia, palette step 800.
    COLOR_FUCHSIA_800 = "color.fuchsia.800": Color = hex("#C844DC");
    /// Fuchsia, palette step 900.
    COLOR_FUCHSIA_900 = "color.fuchsia.900": Color = hex("#B539C8");
    /// Fuchsia, palette step 1000.
    COLOR_FUCHSIA_1000 = "color.fuchsia.1000": Color = hex("#9C28AF");
    /// Fuchsia, palette step 1100.
    COLOR_FUCHSIA_1100 = "color.fuchsia.1100": Color = hex("#871B9A");
    /// Fuchsia, palette step 1200.
    COLOR_FUCHSIA_1200 = "color.fuchsia.1200": Color = hex("#710F83");
    /// Fuchsia, palette step 1300.
    COLOR_FUCHSIA_1300 = "color.fuchsia.1300": Color = hex("#5C046D");
    /// Fuchsia, palette step 1400.
    COLOR_FUCHSIA_1400 = "color.fuchsia.1400": Color = hex("#480058");
    /// Fuchsia, palette step 1500.
    COLOR_FUCHSIA_1500 = "color.fuchsia.1500": Color = hex("#360042");
    /// Fuchsia, palette step 1600.
    COLOR_FUCHSIA_1600 = "color.fuchsia.1600": Color = hex("#1D0023");

    /// Magenta, palette step 100.
    COLOR_MAGENTA_100 = "color.magenta.100": Color = hex("#FFF5F8");
    /// Magenta, palette step 200.
    COLOR_MAGENTA_200 = "color.magenta.200": Color = hex("#FFE8F0");
    /// Magenta, palette step 300.
    COLOR_MAGENTA_300 = "color.magenta.300": Color = hex("#FFD5E3");
    /// Magenta, palette step 400.
    COLOR_MAGENTA_400 = "color.magenta.400": Color = hex("#FFB9D0");
    /// Magenta, palette step 500.
    COLOR_MAGENTA_500 = "color.magenta.500": Color = hex("#FF98BB");
    /// Magenta, palette step 600.
    COLOR_MAGENTA_600 = "color.magenta.600": Color = hex("#FF709F");
    /// Magenta, palette step 700.
    COLOR_MAGENTA_700 = "color.magenta.700": Color = hex("#FF4885");
    /// Magenta, palette step 800.
    COLOR_MAGENTA_800 = "color.magenta.800": Color = hex("#F02D6E");
    /// Magenta, palette step 900.
    COLOR_MAGENTA_900 = "color.magenta.900": Color = hex("#D92361");
    /// Magenta, palette step 1000.
    COLOR_MAGENTA_1000 = "color.magenta.1000": Color = hex("#BA1650");
    /// Magenta, palette step 1100.
    COLOR_MAGENTA_1100 = "color.magenta.1100": Color = hex("#A3053E");
    /// Magenta, palette step 1200.
    COLOR_MAGENTA_1200 = "color.magenta.1200": Color = hex("#880033");
    /// Magenta, palette step 1300.
    COLOR_MAGENTA_1300 = "color.magenta.1300": Color = hex("#6F0028");
    /// Magenta, palette step 1400.
    COLOR_MAGENTA_1400 = "color.magenta.1400": Color = hex("#56001E");
    /// Magenta, palette step 1500.
    COLOR_MAGENTA_1500 = "color.magenta.1500": Color = hex("#400016");
    /// Magenta, palette step 1600.
    COLOR_MAGENTA_1600 = "color.magenta.1600": Color = hex("#23000C");

    /// Pink, palette step 100.
    COLOR_PINK_100 = "color.pink.100": Color = hex("#FFF6FC");
    /// Pink, palette step 200.
    COLOR_PINK_200 = "color.pink.200": Color = hex("#FFE8F7");
    /// Pink, palette step 300.
    COLOR_PINK_300 = "color.pink.300": Color = hex("#FFD3F0");
    /// Pink, palette step 400.
    COLOR_PINK_400 = "color.pink.400": Color = hex("#FFB5E6");
    /// Pink, palette step 500.
    COLOR_PINK_500 = "color.pink.500": Color = hex("#FF94DB");
    /// Pink, palette step 600.
    COLOR_PINK_600 = "color.pink.600": Color = hex("#FF67CC");
    /// Pink, palette step 700.
    COLOR_PINK_700 = "color.pink.700": Color = hex("#F24CB8");
    /// Pink, palette step 800.
    COLOR_PINK_800 = "color.pink.800": Color = hex("#E434A3");
    /// Pink, palette step 900.
    COLOR_PINK_900 = "color.pink.900": Color = hex("#CE2A92");
    /// Pink, palette step 1000.
    COLOR_PINK_1000 = "color.pink.1000": Color = hex("#B01F7B");
    /// Pink, palette step 1100.
    COLOR_PINK_1100 = "color.pink.1100": Color = hex("#981668");
    /// Pink, palette step 1200.
    COLOR_PINK_1200 = "color.pink.1200": Color = hex("#800C55");
    /// Pink, palette step 1300.
    COLOR_PINK_1300 = "color.pink.1300": Color = hex("#690344");
    /// Pink, palette step 1400.
    COLOR_PINK_1400 = "color.pink.1400": Color = hex("#530035");
    /// Pink, palette step 1500.
    COLOR_PINK_1500 = "color.pink.1500": Color = hex("#3E0027");
    /// Pink, palette step 1600.
    COLOR_PINK_1600 = "color.pink.1600": Color = hex("#210015");

    /// Turquoise, palette step 100.
    COLOR_TURQUOISE_100 = "color.turquoise.100": Color = hex("#EEFBFB");
    /// Turquoise, palette step 200.
    COLOR_TURQUOISE_200 = "color.turquoise.200": Color = hex("#D1F5F5");
    /// Turquoise, palette step 300.
    COLOR_TURQUOISE_300 = "color.turquoise.300": Color = hex("#A9ECED");
    /// Turquoise, palette step 400.
    COLOR_TURQUOISE_400 = "color.turquoise.400": Color = hex("#6FDDE4");
    /// Turquoise, palette step 500.
    COLOR_TURQUOISE_500 = "color.turquoise.500": Color = hex("#27CAD8");
    /// Turquoise, palette step 600.
    COLOR_TURQUOISE_600 = "color.turquoise.600": Color = hex("#0FB1C0");
    /// Turquoise, palette step 700.
    COLOR_TURQUOISE_700 = "color.turquoise.700": Color = hex("#0C9EAB");
    /// Turquoise, palette step 800.
    COLOR_TURQUOISE_800 = "color.turquoise.800": Color = hex("#0A8D99");
    /// Turquoise, palette step 900.
    COLOR_TURQUOISE_900 = "color.turquoise.900": Color = hex("#087E89");
    /// Turquoise, palette step 1000.
    COLOR_TURQUOISE_1000 = "color.turquoise.1000": Color = hex("#056B74");
    /// Turquoise, palette step 1100.
    COLOR_TURQUOISE_1100 = "color.turquoise.1100": Color = hex("#035A62");
    /// Turquoise, palette step 1200.
    COLOR_TURQUOISE_1200 = "color.turquoise.1200": Color = hex("#014A51");
    /// Turquoise, palette step 1300.
    COLOR_TURQUOISE_1300 = "color.turquoise.1300": Color = hex("#003B41");
    /// Turquoise, palette step 1400.
    COLOR_TURQUOISE_1400 = "color.turquoise.1400": Color = hex("#002C31");
    /// Turquoise, palette step 1500.
    COLOR_TURQUOISE_1500 = "color.turquoise.1500": Color = hex("#002023");
    /// Turquoise, palette step 1600.
    COLOR_TURQUOISE_1600 = "color.turquoise.1600": Color = hex("#000F11");

    /// Cinnamon, palette step 100.
    COLOR_CINNAMON_100 = "color.cinnamon.100": Color = hex("#FDF7F3");
    /// Cinnamon, palette step 200.
    COLOR_CINNAMON_200 = "color.cinnamon.200": Color = hex("#F9ECE5");
    /// Cinnamon, palette step 300.
    COLOR_CINNAMON_300 = "color.cinnamon.300": Color = hex("#F4DACB");
    /// Cinnamon, palette step 400.
    COLOR_CINNAMON_400 = "color.cinnamon.400": Color = hex("#EDC4AC");
    /// Cinnamon, palette step 500.
    COLOR_CINNAMON_500 = "color.cinnamon.500": Color = hex("#E5AA88");
    /// Cinnamon, palette step 600.
    COLOR_CINNAMON_600 = "color.cinnamon.600": Color = hex("#D4916C");
    /// Cinnamon, palette step 700.
    COLOR_CINNAMON_700 = "color.cinnamon.700": Color = hex("#C67E58");
    /// Cinnamon, palette step 800.
    COLOR_CINNAMON_800 = "color.cinnamon.800": Color = hex("#B86D46");
    /// Cinnamon, palette step 900.
    COLOR_CINNAMON_900 = "color.cinnamon.900": Color = hex("#AA5E38");
    /// Cinnamon, palette step 1000.
    COLOR_CINNAMON_1000 = "color.cinnamon.1000": Color = hex("#934D2B");
    /// Cinnamon, palette step 1100.
    COLOR_CINNAMON_1100 = "color.cinnamon.1100": Color = hex("#803E20");
    /// Cinnamon, palette step 1200.
    COLOR_CINNAMON_1200 = "color.cinnamon.1200": Color = hex("#6E3015");
    /// Cinnamon, palette step 1300.
    COLOR_CINNAMON_1300 = "color.cinnamon.1300": Color = hex("#5C230B");
    /// Cinnamon, palette step 1400.
    COLOR_CINNAMON_1400 = "color.cinnamon.1400": Color = hex("#481906");
    /// Cinnamon, palette step 1500.
    COLOR_CINNAMON_1500 = "color.cinnamon.1500": Color = hex("#341204");
    /// Cinnamon, palette step 1600.
    COLOR_CINNAMON_1600 = "color.cinnamon.1600": Color = hex("#180802");

    /// Brown, palette step 100.
    COLOR_BROWN_100 = "color.brown.100": Color = hex("#FCF7F2");
    /// Brown, palette step 200.
    COLOR_BROWN_200 = "color.brown.200": Color = hex("#F7EEE1");
    /// Brown, palette step 300.
    COLOR_BROWN_300 = "color.brown.300": Color = hex("#EFDDC3");
    /// Brown, palette step 400.
    COLOR_BROWN_400 = "color.brown.400": Color = hex("#E5C89D");
    /// Brown, palette step 500.
    COLOR_BROWN_500 = "color.brown.500": Color = hex("#D6B17B");
    /// Brown, palette step 600.
    COLOR_BROWN_600 = "color.brown.600": Color = hex("#BE9B68");
    /// Brown, palette step 700.
    COLOR_BROWN_700 = "color.brown.700": Color = hex("#AB8A5A");
    /// Brown, palette step 800.
    COLOR_BROWN_800 = "color.brown.800": Color = hex("#9A7B4D");
    /// Brown, palette step 900.
    COLOR_BROWN_900 = "color.brown.900": Color = hex("#8B6D42");
    /// Brown, palette step 1000.
    COLOR_BROWN_1000 = "color.brown.1000": Color = hex("#775B32");
    /// Brown, palette step 1100.
    COLOR_BROWN_1100 = "color.brown.1100": Color = hex("#674C23");
    /// Brown, palette step 1200.
    COLOR_BROWN_1200 = "color.brown.1200": Color = hex("#583D15");
    /// Brown, palette step 1300.
    COLOR_BROWN_1300 = "color.brown.1300": Color = hex("#463111");
    /// Brown, palette step 1400.
    COLOR_BROWN_1400 = "color.brown.1400": Color = hex("#34250D");
    /// Brown, palette step 1500.
    COLOR_BROWN_1500 = "color.brown.1500": Color = hex("#261A09");
    /// Brown, palette step 1600.
    COLOR_BROWN_1600 = "color.brown.1600": Color = hex("#100C04");

    /// Silver, palette step 100.
    COLOR_SILVER_100 = "color.silver.100": Color = hex("#F7F7F7");
    /// Silver, palette step 200.
    COLOR_SILVER_200 = "color.silver.200": Color = hex("#EFEFEF");
    /// Silver, palette step 300.
    COLOR_SILVER_300 = "color.silver.300": Color = hex("#DFDFDF");
    /// Silver, palette step 400.
    COLOR_SILVER_400 = "color.silver.400": Color = hex("#CCCCCC");
    /// Silver, palette step 500.
    COLOR_SILVER_500 = "color.silver.500": Color = hex("#B7B7B7");
    /// Silver, palette step 600.
    COLOR_SILVER_600 = "color.silver.600": Color = hex("#A0A0A0");
    /// Silver, palette step 700.
    COLOR_SILVER_700 = "color.silver.700": Color = hex("#8F8F8F");
    /// Silver, palette step 800.
    COLOR_SILVER_800 = "color.silver.800": Color = hex("#808080");
    /// Silver, palette step 900.
    COLOR_SILVER_900 = "color.silver.900": Color = hex("#727272");
    /// Silver, palette step 1000.
    COLOR_SILVER_1000 = "color.silver.1000": Color = hex("#606060");
    /// Silver, palette step 1100.
    COLOR_SILVER_1100 = "color.silver.1100": Color = hex("#515151");
    /// Silver, palette step 1200.
    COLOR_SILVER_1200 = "color.silver.1200": Color = hex("#424242");
    /// Silver, palette step 1300.
    COLOR_SILVER_1300 = "color.silver.1300": Color = hex("#343434");
    /// Silver, palette step 1400.
    COLOR_SILVER_1400 = "color.silver.1400": Color = hex("#272727");
    /// Silver, palette step 1500.
    COLOR_SILVER_1500 = "color.silver.1500": Color = hex("#1C1C1C");
    /// Silver, palette step 1600.
    COLOR_SILVER_1600 = "color.silver.1600": Color = hex("#0C0C0C");
}

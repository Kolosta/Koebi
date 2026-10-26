//! Interface unit and scale factors, alignments and the border switch.

use super::*;
use koebi_design::define::*;

koebi_design::tokens! {
    group LAYOUT: Primitive, start = SCALE::END;

    /// Base unit of the user interface: the height of standard controls.
    UI_UNIT = "ui-unit": Dimension = px(20.0), range(8.0, 64.0);

    /// Interface scale factor of 75 %.
    SCALE_75 = "scale.75": Number = 0.75, range(0.5, 4.0);
    /// Interface scale factor of 100 %.
    SCALE_100 = "scale.100": Number = 1.0, range(0.5, 4.0);
    /// Interface scale factor of 125 %.
    SCALE_125 = "scale.125": Number = 1.25, range(0.5, 4.0);
    /// Interface scale factor of 150 %.
    SCALE_150 = "scale.150": Number = 1.5, range(0.5, 4.0);

    /// Alignment at the start (left or top).
    ALIGN_START = "align.start": Number = 0.0, range(0.0, 1.0);
    /// Centered alignment.
    ALIGN_CENTER = "align.center": Number = 0.5, range(0.0, 1.0);
    /// Alignment at the end (right or bottom).
    ALIGN_END = "align.end": Number = 1.0, range(0.0, 1.0);

    /// Whether controls draw borders: 1 draws them, 0 does not.
    BORDER_ENABLED = "border.enabled": Integer = 1, range(0.0, 1.0);
}

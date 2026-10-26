//! The units expressions know: lengths, angles and times, each with its size in the base unit
//! of its dimension (meters, radians, seconds); pixels depend on the pixels per inch of the
//! [`Context`](super::Context).

/// What a unit measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dimension {
    /// Lengths: meters, inches, pixels.
    Length,
    /// Angles: radians, degrees, turns.
    Angle,
    /// Durations: seconds, milliseconds.
    Time,
}

impl Dimension {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Length => 0,
            Self::Angle => 1,
            Self::Time => 2,
        }
    }
}

/// How big a unit is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Size {
    /// This many base units (meters, radians, seconds).
    Fixed(f64),
    /// This many pixels: an inch divided by the pixels per inch, each.
    Pixels(f64),
}

/// A unit: the symbol shown after values, the names typed for it, its dimension and size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit {
    /// The symbol values are shown with (`mm`, `°`).
    pub symbol: &'static str,
    /// What can be typed for it, the symbol first.
    pub names: &'static [&'static str],
    /// What it measures.
    pub dimension: Dimension,
    pub(crate) size: Size,
}

impl Unit {
    /// The size of the unit in the base unit of its dimension, with `pixels_per_inch` pixels
    /// in an inch.
    pub fn base(&self, pixels_per_inch: f64) -> f64 {
        const INCH: f64 = 0.0254;
        match self.size {
            Size::Fixed(size) => size,
            Size::Pixels(count) => INCH * count / pixels_per_inch,
        }
    }
}

/// Pixels: the unit of positions and sizes on the canvas.
pub const PIXEL: Unit = Unit {
    symbol: "px",
    names: &["px", "pixel", "pixels"],
    dimension: Dimension::Length,
    size: Size::Pixels(1.0),
};

/// Millimeters.
pub const MILLIMETER: Unit = Unit {
    symbol: "mm",
    names: &[
        "mm",
        "millimeter",
        "millimeters",
        "millimetre",
        "millimetres",
    ],
    dimension: Dimension::Length,
    size: Size::Fixed(0.001),
};

/// Degrees.
pub const DEGREE: Unit = Unit {
    symbol: "\u{b0}",
    names: &["\u{b0}", "deg", "degree", "degrees"],
    dimension: Dimension::Angle,
    size: Size::Fixed(core::f64::consts::PI / 180.0),
};

/// Radians.
pub const RADIAN: Unit = Unit {
    symbol: "rad",
    names: &["rad", "radian", "radians"],
    dimension: Dimension::Angle,
    size: Size::Fixed(1.0),
};

/// Seconds.
pub const SECOND: Unit = Unit {
    symbol: "s",
    names: &["s", "sec", "second", "seconds"],
    dimension: Dimension::Time,
    size: Size::Fixed(1.0),
};

/// Every unit expressions know.
pub const UNITS: &[Unit] = &[
    PIXEL,
    Unit {
        symbol: "pt",
        names: &["pt", "point", "points"],
        dimension: Dimension::Length,
        size: Size::Fixed(0.0254 / 72.0),
    },
    MILLIMETER,
    Unit {
        symbol: "cm",
        names: &[
            "cm",
            "centimeter",
            "centimeters",
            "centimetre",
            "centimetres",
        ],
        dimension: Dimension::Length,
        size: Size::Fixed(0.01),
    },
    Unit {
        symbol: "m",
        names: &["m", "meter", "meters", "metre", "metres"],
        dimension: Dimension::Length,
        size: Size::Fixed(1.0),
    },
    Unit {
        symbol: "in",
        names: &["in", "inch", "inches", "\""],
        dimension: Dimension::Length,
        size: Size::Fixed(0.0254),
    },
    DEGREE,
    RADIAN,
    Unit {
        symbol: "turn",
        names: &["turn", "turns", "tr"],
        dimension: Dimension::Angle,
        size: Size::Fixed(core::f64::consts::TAU),
    },
    SECOND,
    Unit {
        symbol: "ms",
        names: &["ms", "millisecond", "milliseconds"],
        dimension: Dimension::Time,
        size: Size::Fixed(0.001),
    },
    Unit {
        symbol: "min",
        names: &["min", "minute", "minutes"],
        dimension: Dimension::Time,
        size: Size::Fixed(60.0),
    },
];

/// The unit typed as `name`, if any.
pub fn unit(name: &str) -> Option<&'static Unit> {
    UNITS.iter().find(|unit| unit.names.contains(&name))
}

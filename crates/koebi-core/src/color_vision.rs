//! Color vision deficiencies: simulation and correction matrices.
//!
//! About 8 % of men and 0.5 % of women have a color vision deficiency (CVD). The three cone
//! types of the retina respond to long (L), medium (M) and short (S) wavelengths; a deficiency
//! weakens or removes one of them:
//!
//! | [`Deficiency`] | Missing or shifted cone | Colors confused |
//! |---|---|---|
//! | [`Protan`](Deficiency::Protan) | L (red) | red with green, red looks darker |
//! | [`Deutan`](Deficiency::Deutan) | M (green) | red with green, the most common form |
//! | [`Tritan`](Deficiency::Tritan) | S (blue) | blue with green, yellow with violet |
//!
//! Every transformation here is a 3×3 [`ColorMatrix`] applied in **linear-light** sRGB:
//!
//! - [`Deficiency::simulation`] shows how colors appear to someone with the deficiency
//!   (Machado, Oliveira and Fernandes, *A Physiologically-based Model for Simulation of Color
//!   Vision Deficiency*, IEEE TVCG 2009). Designers use it to check their work.
//! - [`Deficiency::correction`] (daltonization) moves the information a viewer cannot see
//!   into channels they can see, so that confused colors become distinguishable again.
//! - [`achromatopsia`] simulates total color blindness (luminance only).
//!
//! Every matrix maps white to white and grays to themselves, so neutral UI surfaces never
//! change. Matrices are meant for UI colors: canvas content keeps its own color management.

use crate::color::{Color, LinearRgba};

/// A linear transformation of colors in linear-light sRGB.
///
/// `apply` converts a gamma-encoded [`Color`] to linear light, multiplies it by the matrix
/// (rows are output channels), clamps the result to the displayable range and encodes it
/// again. Opacity is unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorMatrix {
    rows: [[f32; 3]; 3],
}

impl ColorMatrix {
    /// The matrix that changes nothing.
    pub const IDENTITY: Self = Self::new([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    /// Creates a matrix from its rows: `out.r = rows[0] · (r, g, b)`, and so on.
    pub const fn new(rows: [[f32; 3]; 3]) -> Self {
        Self { rows }
    }

    /// The rows of the matrix.
    pub const fn rows(&self) -> [[f32; 3]; 3] {
        self.rows
    }

    /// `true` when the matrix is exactly the identity.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    /// `true` when every coefficient is finite.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.rows.iter().flatten().all(|c| c.is_finite())
    }

    /// The matrix that applies `self`, then `next`.
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        let (a, b) = (self.rows, next.rows);
        let mut out = [[0.0_f32; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = b[i][0].mul_add(a[0][j], b[i][1].mul_add(a[1][j], b[i][2] * a[2][j]));
            }
        }
        Self::new(out)
    }

    /// Interpolates every coefficient from `self` (`t = 0`) to `other` (`t = 1`).
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let mut out = self.rows;
        for (row, target) in out.iter_mut().zip(other.rows) {
            for (cell, end) in row.iter_mut().zip(target) {
                *cell = (end - *cell).mul_add(t, *cell);
            }
        }
        Self::new(out)
    }

    /// Transforms a linear color without clamping.
    #[must_use]
    pub fn apply_linear(&self, color: LinearRgba) -> LinearRgba {
        let [r, g, b] = self
            .rows
            .map(|[x, y, z]| x.mul_add(color.r, y.mul_add(color.g, z * color.b)));
        LinearRgba::new(r, g, b, color.a)
    }

    /// Transforms a gamma-encoded color, clamping the result to the sRGB gamut.
    #[must_use]
    pub fn apply(&self, color: Color) -> Color {
        if self.is_identity() {
            return color;
        }
        let out = self.apply_linear(color.to_linear());
        LinearRgba::new(
            out.r.clamp(0.0, 1.0),
            out.g.clamp(0.0, 1.0),
            out.b.clamp(0.0, 1.0),
            out.a,
        )
        .to_srgb()
    }
}

impl Default for ColorMatrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// A type of color vision deficiency, named after the affected cone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Deficiency {
    /// Long-wavelength (red) cones: protanomaly, or protanopia at full severity.
    Protan,
    /// Medium-wavelength (green) cones: deuteranomaly, or deuteranopia at full severity.
    Deutan,
    /// Short-wavelength (blue) cones: tritanomaly, or tritanopia at full severity.
    Tritan,
}

impl Deficiency {
    /// Every deficiency, in a stable order.
    pub const ALL: [Self; 3] = [Self::Protan, Self::Deutan, Self::Tritan];

    /// Simulates the deficiency. `severity` goes from `0.0` (normal vision) to `1.0`
    /// (dichromacy: protanopia, deuteranopia or tritanopia) and is clamped to that range.
    ///
    /// Full severity uses the published matrices of Machado et al. (2009). Lower severities
    /// interpolate linearly between the identity and that matrix, a close approximation of the
    /// paper's anomalous trichromacy matrices.
    #[must_use]
    pub fn simulation(self, severity: f32) -> ColorMatrix {
        ColorMatrix::IDENTITY.lerp(self.dichromat(), unit(severity))
    }

    /// Corrects the deficiency (daltonization). `strength` goes from `0.0` (no change) to
    /// `1.0` and is clamped to that range.
    ///
    /// The error between a color and its simulation (what the viewer misses) is redistributed
    /// to channels the viewer perceives: `M = I + strength · E · (I − S)`, where `S` is the
    /// full-severity simulation and `E` the error-redistribution matrix of Fidaner, Lin and
    /// Ozguven (2005). Grays and white are unchanged.
    #[must_use]
    pub fn correction(self, strength: f32) -> ColorMatrix {
        let s = self.dichromat().rows;
        let e = self.error_redistribution();
        let k = unit(strength);
        let identity = ColorMatrix::IDENTITY.rows;
        let mut rows = identity;
        for (i, row) in rows.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                // (I - S)[m][j]: what the viewer misses of input channel j in channel m.
                let missing = |m: usize| identity[m][j] - s[m][j];
                let error = e[i][0].mul_add(
                    missing(0),
                    e[i][1].mul_add(missing(1), e[i][2] * missing(2)),
                );
                *cell = k.mul_add(error, *cell);
            }
        }
        ColorMatrix::new(rows)
    }

    /// Machado et al. (2009), severity 1.0, for linear RGB.
    const fn dichromat(self) -> ColorMatrix {
        ColorMatrix::new(match self {
            Self::Protan => [
                [0.152_286, 1.052_583, -0.204_868],
                [0.114_503, 0.786_281, 0.099_216],
                [-0.003_882, -0.048_116, 1.051_998],
            ],
            Self::Deutan => [
                [0.367_322, 0.860_646, -0.227_968],
                [0.280_085, 0.672_501, 0.047_413],
                [-0.011_820, 0.042_940, 0.968_881],
            ],
            Self::Tritan => [
                [1.255_528, -0.076_749, -0.178_779],
                [-0.078_411, 0.930_809, 0.147_602],
                [0.004_733, 0.691_367, 0.303_900],
            ],
        })
    }

    /// Where the invisible part of a color goes: red-green errors into green and blue,
    /// blue-yellow errors into red and green.
    const fn error_redistribution(self) -> [[f32; 3]; 3] {
        match self {
            Self::Protan | Self::Deutan => [[0.0, 0.0, 0.0], [0.7, 1.0, 0.0], [0.7, 0.0, 1.0]],
            Self::Tritan => [[1.0, 0.0, 0.7], [0.0, 1.0, 0.7], [0.0, 0.0, 0.0]],
        }
    }
}

/// Simulates achromatopsia (no functional cones: luminance only). `severity` goes from `0.0`
/// to `1.0` and is clamped to that range.
#[must_use]
pub fn achromatopsia(severity: f32) -> ColorMatrix {
    // Rec. 709 luminance, the weights of WCAG relative luminance.
    const Y: [f32; 3] = [0.2126, 0.7152, 0.0722];
    ColorMatrix::IDENTITY.lerp(ColorMatrix::new([Y, Y, Y]), unit(severity))
}

fn unit(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_rgb_close(actual: Color, expected: Color, tolerance: f32) {
        let pairs = [
            (actual.r, expected.r),
            (actual.g, expected.g),
            (actual.b, expected.b),
        ];
        assert!(
            pairs.iter().all(|(a, e)| (a - e).abs() <= tolerance),
            "expected {expected}, got {actual}"
        );
    }

    fn all_matrices() -> Vec<ColorMatrix> {
        let mut all = vec![achromatopsia(1.0), achromatopsia(0.4)];
        for deficiency in Deficiency::ALL {
            for amount in [0.3, 1.0] {
                all.push(deficiency.simulation(amount));
                all.push(deficiency.correction(amount));
            }
        }
        all
    }

    #[test]
    fn color_matrix_identity_is_exact() {
        let color = Color::from_hex(0x33_66_FF).with_alpha(0.5);
        assert_eq!(ColorMatrix::IDENTITY.apply(color), color);
        assert!(ColorMatrix::default().is_identity());
    }

    #[test]
    fn color_matrix_then_composes_in_order() {
        let swap_rg = ColorMatrix::new([[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
        let red_only = ColorMatrix::new([[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]);
        // Swap first: green moves to red, then only red survives.
        let combined = swap_rg.then(red_only);
        let out = combined.apply_linear(LinearRgba::new(0.2, 0.7, 0.4, 1.0));
        assert_eq!((out.r, out.g, out.b), (0.7, 0.0, 0.0));
    }

    #[test]
    fn simulations_match_the_published_rows() {
        // Machado et al. (2009), severity 1.0, first row of the protanopia matrix.
        let rows = Deficiency::Protan.simulation(1.0).rows();
        assert_eq!(rows[0], [0.152_286, 1.052_583, -0.204_868]);
        // Severity 0 is normal vision.
        assert!(Deficiency::Deutan.simulation(0.0).is_identity());
        assert_eq!(
            Deficiency::Tritan.simulation(2.0),
            Deficiency::Tritan.simulation(1.0)
        );
    }

    #[test]
    fn every_matrix_keeps_grays_and_opacity() {
        for matrix in all_matrices() {
            for gray in [0x00_00_00, 0x80_80_80, 0xFF_FF_FF] {
                let color = Color::from_hex(gray).with_alpha(0.25);
                let out = matrix.apply(color);
                assert_rgb_close(out, color, 2e-3);
                assert!((out.a - 0.25).abs() < f32::EPSILON);
            }
        }
    }

    #[test]
    fn protanopia_confuses_red_and_green() {
        let simulate = Deficiency::Protan.simulation(1.0);
        let (red, green) = (Color::from_hex(0xE0_40_40), Color::from_hex(0x6E_8E_3A));
        let chroma_distance = |a: Color, b: Color| {
            let (a, b) = (a.to_oklab(), b.to_oklab());
            (a.a - b.a).hypot(a.b - b.b)
        };
        let normal = chroma_distance(red, green);
        let simulated = chroma_distance(simulate.apply(red), simulate.apply(green));
        assert!(
            simulated < normal * 0.5,
            "simulated {simulated} vs normal {normal}"
        );
    }

    #[test]
    fn correction_makes_confused_colors_distinguishable_again() {
        let pairs = [
            (Deficiency::Protan, 0xD0_30_30, 0x50_90_30),
            (Deficiency::Deutan, 0xD0_30_30, 0x50_90_30),
            (Deficiency::Tritan, 0x30_60_D0, 0x30_A0_90),
        ];
        for (deficiency, a, b) in pairs {
            let simulate = deficiency.simulation(1.0);
            let correct = deficiency.correction(1.0);
            let seen = |c: Color| simulate.apply(c).to_oklab();
            let distance =
                |x: crate::Oklab, y: crate::Oklab| (x.l - y.l).hypot((x.a - y.a).hypot(x.b - y.b));
            let (a, b) = (Color::from_hex(a), Color::from_hex(b));
            let plain = distance(seen(a), seen(b));
            let corrected = distance(seen(correct.apply(a)), seen(correct.apply(b)));
            assert!(
                corrected > plain,
                "{deficiency:?}: corrected {corrected} <= uncorrected {plain}"
            );
        }
    }

    #[test]
    fn achromatopsia_outputs_luminance() {
        let out = achromatopsia(1.0).apply(Color::from_hex(0x33_99_CC));
        assert_rgb_close(out, Color::new(out.r, out.r, out.r, 1.0), 1e-6);
        let luminance = Color::from_hex(0x33_99_CC).relative_luminance();
        assert!((out.to_linear().r - luminance).abs() < 1e-3);
    }

    #[test]
    fn amounts_are_clamped_and_nan_safe() {
        assert!(Deficiency::Protan.correction(f32::NAN).is_identity());
        assert!(achromatopsia(-1.0).is_identity());
        assert!(all_matrices().iter().all(ColorMatrix::is_finite));
    }
}

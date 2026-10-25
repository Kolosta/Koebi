//! The color vision layer: a color-only transformation on top of themes and overrides.
//!
//! The transformation is a 3×3 matrix in linear sRGB from [`koebi_core::color_vision`],
//! applied once per color token when the token graph is resolved, never per pixel. Tokens
//! whose layer mask lacks [`Layers::COLOR_VISION`](crate::Layers::COLOR_VISION) keep their
//! colors, and so does everything aliasing them: a logo color or a color swatch stays true.
//! Canvas content never goes through this layer.

use koebi_core::color_vision::{ColorMatrix, Deficiency, achromatopsia};

use crate::error::{Error, Result};

/// The color vision setting.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColorVision {
    /// No transformation.
    #[default]
    Normal,
    /// Corrects a deficiency (daltonization) so that confused colors differ again.
    Correct {
        /// The deficiency to correct.
        deficiency: Deficiency,
        /// From `0.0` (no change) to `1.0`.
        strength: f32,
    },
    /// Shows colors as a person with a deficiency sees them, to check a design.
    Simulate {
        /// The deficiency to simulate.
        deficiency: Deficiency,
        /// From `0.0` (normal vision) to `1.0` (dichromacy).
        severity: f32,
    },
    /// Shows colors without any hue, as with achromatopsia.
    SimulateAchromatopsia {
        /// From `0.0` (normal vision) to `1.0` (luminance only).
        severity: f32,
    },
    /// Any linear transformation, for experiments or other conditions.
    Custom(ColorMatrix),
}

impl ColorVision {
    /// The matrix applied to color tokens.
    pub fn matrix(&self) -> ColorMatrix {
        match *self {
            Self::Normal => ColorMatrix::IDENTITY,
            Self::Correct {
                deficiency,
                strength,
            } => deficiency.correction(strength),
            Self::Simulate {
                deficiency,
                severity,
            } => deficiency.simulation(severity),
            Self::SimulateAchromatopsia { severity } => achromatopsia(severity),
            Self::Custom(matrix) => matrix,
        }
    }

    /// Checks that amounts lie in `0.0..=1.0` and custom matrices are finite.
    pub(crate) fn validate(&self) -> Result<()> {
        let amount = match *self {
            Self::Normal => return Ok(()),
            Self::Custom(matrix) => {
                return if matrix.is_finite() {
                    Ok(())
                } else {
                    Err(Error::InvalidColorVision("custom matrices must be finite"))
                };
            }
            Self::Correct { strength, .. } => strength,
            Self::Simulate { severity, .. } | Self::SimulateAchromatopsia { severity } => severity,
        };
        if (0.0..=1.0).contains(&amount) {
            Ok(())
        } else {
            Err(Error::InvalidColorVision("amounts lie in 0..=1"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_vision_is_the_identity() {
        assert!(ColorVision::Normal.matrix().is_identity());
        assert!(ColorVision::default().matrix().is_identity());
    }

    #[test]
    fn settings_map_to_core_matrices() {
        let correct = ColorVision::Correct {
            deficiency: Deficiency::Deutan,
            strength: 1.0,
        };
        assert_eq!(correct.matrix(), Deficiency::Deutan.correction(1.0));
        let simulate = ColorVision::SimulateAchromatopsia { severity: 1.0 };
        assert_eq!(simulate.matrix(), achromatopsia(1.0));
    }

    #[test]
    fn invalid_settings_are_rejected() {
        let too_strong = ColorVision::Correct {
            deficiency: Deficiency::Protan,
            strength: 1.5,
        };
        assert!(too_strong.validate().is_err());
        let nan = ColorVision::Simulate {
            deficiency: Deficiency::Tritan,
            severity: f32::NAN,
        };
        assert!(nan.validate().is_err());
        let infinite = ColorVision::Custom(ColorMatrix::new([[f32::INFINITY; 3]; 3]));
        assert!(infinite.validate().is_err());
        assert!(ColorVision::Normal.validate().is_ok());
    }
}

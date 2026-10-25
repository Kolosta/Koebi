//! Design changes asked on the command line (`--theme`, `--override`, `--color-vision`), and
//! the one-line summary of the design state logged at startup.

use core::time::Duration;
use std::sync::Arc;

use koebi_core::Color;
use koebi_design::kind::KindId;
use koebi_design::value::{CubicBezier, Definition, FontWeight, Value};
use koebi_design::{ColorVision, Deficiency, DesignSystem, Scope};

use crate::options::Options;

/// The color vision settings offered by the command line and the gallery, with their names.
pub(crate) const COLOR_VISION_MODES: [(&str, ColorVision); 8] = [
    ("normal", ColorVision::Normal),
    ("simulate-protan", simulate(Deficiency::Protan)),
    ("simulate-deutan", simulate(Deficiency::Deutan)),
    ("simulate-tritan", simulate(Deficiency::Tritan)),
    (
        "simulate-achromatopsia",
        ColorVision::SimulateAchromatopsia { severity: 1.0 },
    ),
    ("correct-protan", correct(Deficiency::Protan)),
    ("correct-deutan", correct(Deficiency::Deutan)),
    ("correct-tritan", correct(Deficiency::Tritan)),
];

const fn simulate(deficiency: Deficiency) -> ColorVision {
    ColorVision::Simulate {
        deficiency,
        severity: 1.0,
    }
}

const fn correct(deficiency: Deficiency) -> ColorVision {
    ColorVision::Correct {
        deficiency,
        strength: 1.0,
    }
}

/// Applies the command line's design changes on top of the restored state: the theme, then
/// the overrides (global), then the color vision setting.
///
/// # Errors
///
/// A message naming the option and the problem: an unknown theme or token, a value that does
/// not parse, or a change the design system rejects. Changes applied before the faulty one
/// stay applied; the application exits without saving them.
pub(crate) fn apply(design: &mut DesignSystem, options: &Options) -> Result<(), String> {
    if let Some(key) = &options.theme {
        let theme = design.find_theme(key).ok_or_else(|| {
            let keys: Vec<_> = design.themes().map(|(_, theme)| theme.key()).collect();
            format!(
                "--theme: unknown theme {key:?} (themes: {})",
                keys.join(", ")
            )
        })?;
        design
            .set_active_theme(theme)
            .map_err(|error| format!("--theme {key}: {error}"))?;
    }
    for (path, text) in &options.overrides {
        let context = |error: String| format!("--override {path}={text}: {error}");
        let token = design
            .find(path)
            .ok_or_else(|| context("unknown token".into()))?;
        let definition = parse_definition(design, token, text).map_err(context)?;
        design
            .set_override(Scope::Global, token, definition)
            .map_err(|error| context(error.to_string()))?;
    }
    if let Some(vision) = options.color_vision {
        design
            .set_color_vision(vision)
            .map_err(|error| format!("--color-vision: {error}"))?;
    }
    Ok(())
}

/// Parses `text` as a definition of `token`: `{path}` aliases another token, anything else is
/// a value of the token's kind (`#rrggbb` colors, numbers with an optional `px` or `ms` unit,
/// `x1,y1,x2,y2` curves, font weights from 1 to 1000, font family names).
pub(crate) fn parse_definition(
    design: &DesignSystem,
    token: koebi_design::TokenId,
    text: &str,
) -> Result<Definition, String> {
    if let Some(path) = text.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        return design
            .find(path)
            .map(Definition::Alias)
            .ok_or_else(|| format!("unknown alias target {{{path}}}"));
    }
    let kind = design
        .info(token)
        .map(koebi_design::schema::TokenInfo::kind)
        .ok_or_else(|| "unknown token".to_owned())?;
    let number = |unit: &str| {
        let digits = text.strip_suffix(unit).unwrap_or(text).trim();
        digits
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("expected a number, found {text:?}"))
    };
    let value = match kind {
        KindId::Color => Value::Color(text.parse::<Color>().map_err(|error| error.to_string())?),
        KindId::Dimension => Value::Dimension(number("px")?),
        KindId::Number => Value::Number(number("")?),
        KindId::Integer => Value::Integer(
            text.parse()
                .map_err(|_| format!("expected an integer, found {text:?}"))?,
        ),
        KindId::Duration => {
            let milliseconds = number("ms")?;
            Value::Duration(
                Duration::try_from_secs_f32(milliseconds / 1000.0)
                    .map_err(|_| format!("expected a duration in milliseconds, found {text:?}"))?,
            )
        }
        KindId::CubicBezier => {
            let parts: Vec<f32> = text
                .split(',')
                .map(|part| part.trim().parse::<f32>())
                .collect::<Result<_, _>>()
                .map_err(|_| format!("expected x1,y1,x2,y2, found {text:?}"))?;
            let [x1, y1, x2, y2] = parts[..] else {
                return Err(format!("expected x1,y1,x2,y2, found {text:?}"));
            };
            Value::CubicBezier(CubicBezier::new(x1, y1, x2, y2))
        }
        KindId::FontWeight => Value::FontWeight(
            text.parse()
                .ok()
                .and_then(FontWeight::new)
                .ok_or_else(|| format!("expected a font weight from 1 to 1000, found {text:?}"))?,
        ),
        KindId::FontFamily => Value::FontFamily(Arc::from(text)),
    };
    Ok(Definition::Value(value))
}

/// The name of a color vision setting, as the command line spells it.
pub(crate) fn color_vision_name(vision: ColorVision) -> &'static str {
    COLOR_VISION_MODES
        .iter()
        .find(|(_, mode)| *mode == vision)
        .map_or("custom", |(name, _)| name)
}

/// One line describing the design state: active theme, overrides and color vision.
pub(crate) fn summary(design: &DesignSystem) -> String {
    let count = |scope| design.overrides(scope).map_or(0, Iterator::count);
    let theme = design
        .theme(design.active_theme())
        .map_or("?", |theme| theme.key());
    format!(
        "design: theme {theme}, {} global override(s), {} theme override(s), color vision {}",
        count(Scope::Global),
        count(Scope::ActiveTheme),
        color_vision_name(design.color_vision())
    )
}

#[cfg(test)]
mod tests {
    use koebi_tokens::{KOEBI, component, semantic};

    use super::*;

    fn design() -> DesignSystem {
        DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
    }

    fn options(args: &[&str]) -> Options {
        Options::parse(args).expect("valid arguments")
    }

    #[test]
    fn command_line_changes_reach_the_design_system() {
        let mut design = design();
        let args = options(&[
            "--theme",
            "light",
            "--override",
            "semantic.radius.control=7px",
            "--override",
            "component.button.background.color.default=#ff8800",
            "--override",
            "semantic.accent.color.700={primitive.color.green.700}",
            "--color-vision",
            "correct-tritan",
        ]);
        apply(&mut design, &args).unwrap();
        assert_eq!(design.theme(design.active_theme()).unwrap().key(), "light");
        assert_eq!(design.get(semantic::RADIUS_CONTROL), 7.0);
        assert_eq!(
            design.override_of(Scope::Global, component::BUTTON_BACKGROUND_COLOR_DEFAULT),
            Some(&Definition::Value(Value::Color(Color::from_hex(
                0xFF_88_00
            ))))
        );
        assert_eq!(
            summary(&design),
            "design: theme light, 3 global override(s), 0 theme override(s), color vision \
             correct-tritan"
        );
    }

    #[test]
    fn values_parse_according_to_the_token_kind() {
        let design = design();
        let parse = |path: &str, text: &str| {
            parse_definition(&design, design.find(path).expect("known token"), text)
        };
        assert_eq!(
            parse("semantic.animation.duration.enter", "250ms"),
            Ok(Definition::Value(Value::Duration(Duration::from_millis(
                250
            ))))
        );
        assert_eq!(
            parse("semantic.layer.modal", "12"),
            Ok(Definition::Value(Value::Integer(12)))
        );
        assert!(parse("semantic.radius.control", "wide").is_err());
        assert!(parse("semantic.radius.control", "{semantic.nowhere}").is_err());
        assert!(parse("semantic.font.weight.body.m", "1200").is_err());
    }

    #[test]
    fn bad_options_are_reported_with_their_option() {
        let mut design = design();
        let error = apply(&mut design, &options(&["--theme", "sepia"])).unwrap_err();
        assert!(
            error.starts_with("--theme: unknown theme \"sepia\""),
            "{error}"
        );
        let error = apply(
            &mut design,
            &options(&["--override", "semantic.radius.control=99"]),
        )
        .unwrap_err();
        assert!(
            error.starts_with("--override semantic.radius.control=99: "),
            "{error}"
        );
    }

    #[test]
    fn color_vision_modes_have_unique_names() {
        for (name, vision) in COLOR_VISION_MODES {
            assert_eq!(color_vision_name(vision), name);
        }
    }
}

//! Contrast of every component in every built-in theme (`docs/design-system/color.md`).
//!
//! Content is paired with what it is drawn on, state by state, from the token paths
//! themselves: `<component>.label.color.<state>` on `<component>.background.color.<state>`,
//! and the same for icons and marks. A new component, state or theme is checked without
//! touching this file.
//!
//! | Content | Required | High contrast |
//! |---|---|---|
//! | Text (labels, values, placeholders) | 4.5:1 (WCAG 1.4.3) | 7:1 (1.4.6) |
//! | Icons, marks, focus rings, boundaries | 3:1 (1.4.11) | 4.5:1 |
//!
//! Disabled content is exempt (WCAG), and stays in the high-contrast theme's 4.5:1 for icons.

// Token constants of the last groups sit at the end of a chain of group starts: evaluating
// them goes deeper than the default limit on some hosts.
#![recursion_limit = "256"]

use koebi_core::Color;
use koebi_design::kind::KindId;
use koebi_design::schema::TokenInfo;
use koebi_design::value::Value;
use koebi_design::{DesignSystem, ThemeId};
use koebi_tokens::{KOEBI, component, semantic};

fn design() -> DesignSystem {
    DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
}

fn themes(design: &DesignSystem) -> Vec<(ThemeId, String)> {
    design
        .themes()
        .map(|(id, theme)| (id, theme.key().to_owned()))
        .collect()
}

/// A token's path, for messages.
fn name(design: &DesignSystem, token: impl Into<koebi_design::TokenId>) -> String {
    design
        .info(token)
        .map_or_else(String::new, |info| info.qualified().to_string())
}

fn color(design: &DesignSystem, path: &str) -> Option<Color> {
    let token = design.find(path)?;
    match design.value(token)? {
        Value::Color(color) => Some(*color),
        _ => None,
    }
}

/// The content and the fill paths of every component, paired by state: (content, fill,
/// whether the content is text).
fn pairs() -> Vec<(String, String, bool)> {
    let paths: Vec<String> = KOEBI
        .tokens()
        .filter(|(_, info): &(_, &TokenInfo)| info.kind() == KindId::Color)
        .map(|(_, info)| info.qualified().to_string())
        .filter(|path| path.starts_with("component."))
        .collect();
    let mut pairs = Vec::new();
    for content in &paths {
        for (part, text) in [
            (".label.color.", true),
            (".icon.color.", false),
            (".mark.color.", false),
        ] {
            let Some(at) = content.find(part) else {
                continue;
            };
            let (prefix, state) = (&content[..at], &content[at + part.len()..]);
            if state.contains("disabled") {
                continue;
            }
            // The fill of the same state, or the component's default fill.
            let fill = [state, "default"]
                .iter()
                .map(|state| format!("{prefix}.background.color.{state}"))
                .find(|fill| paths.contains(fill));
            if let Some(fill) = fill {
                pairs.push((content.clone(), fill, text));
            }
        }
    }
    pairs
}

#[test]
fn every_component_s_content_reads_on_its_fill_in_every_theme() {
    let mut design = design();
    let pairs = pairs();
    assert!(pairs.len() > 30, "{} pairs", pairs.len());
    let mut failures = Vec::new();
    for (id, key) in themes(&design) {
        design.set_active_theme(id).unwrap();
        let high = key == "high-contrast";
        for (content, fill, text) in &pairs {
            let (Some(ink), Some(below)) = (color(&design, content), color(&design, fill)) else {
                continue;
            };
            // A transparent fill shows what lies below: the surface it sits on.
            let surface = color(&design, "semantic.background.raised.default").unwrap();
            let below = below.over(surface);
            let ratio = ink.over(below).contrast_ratio(below);
            let needed = match (text, high) {
                (true, true) => 7.0,
                (false, false) => 3.0,
                // Text in a normal theme, icons in the high-contrast one.
                _ => 4.5,
            };
            if ratio < needed {
                failures.push(format!("{key}: {content} on {fill}: {ratio:.2} < {needed}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn rows_hold_their_text_in_every_state_in_every_theme() {
    let mut design = design();
    let mut failures = Vec::new();
    for (id, key) in themes(&design) {
        design.set_active_theme(id).unwrap();
        let needed = if key == "high-contrast" { 7.0 } else { 4.5 };
        let text = design.get(component::OUTLINER_ROW_TEXT_DEFAULT);
        for row in [
            component::OUTLINER_ROW_BACKGROUND_HOVER,
            component::OUTLINER_ROW_BACKGROUND_SELECTED,
            component::OUTLINER_ROW_BACKGROUND_SELECTED_HOVER,
            component::OUTLINER_ROW_BACKGROUND_ACTIVE,
            component::OUTLINER_ROW_BACKGROUND_ACTIVE_HOVER,
        ] {
            let ratio = text.contrast_ratio(design.get(row));
            if ratio < needed {
                failures.push(format!(
                    "{key}: row text on {}: {ratio:.2} < {needed}",
                    name(&design, row)
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{}",
        failures.join(
            "
"
        )
    );
}

#[test]
fn text_and_icons_read_on_every_surface_in_every_theme() {
    let mut design = design();
    let surfaces = [
        semantic::BACKGROUND_BASE_DEFAULT,
        semantic::BACKGROUND_RAISED_DEFAULT,
        semantic::BACKGROUND_SUNKEN_DEFAULT,
        semantic::BACKGROUND_FLOATING_DEFAULT,
        semantic::BACKGROUND_SURFACE_CANVAS,
        semantic::BACKGROUND_APP_CONTROL,
        semantic::BACKGROUND_APP_CHILD,
        semantic::BACKGROUND_APP_FRAME,
        semantic::BACKGROUND_APP_FRAME_HOVER,
    ];
    let mut failures = Vec::new();
    for (id, key) in themes(&design) {
        design.set_active_theme(id).unwrap();
        let high = key == "high-contrast";
        let checks = [
            (
                semantic::TEXT_COLOR_PRIMARY_DEFAULT,
                if high { 7.0 } else { 4.5 },
            ),
            (
                semantic::TEXT_COLOR_SECONDARY_DEFAULT,
                if high { 7.0 } else { 4.5 },
            ),
            (
                semantic::TEXT_COLOR_TERTIARY_DEFAULT,
                if high { 7.0 } else { 4.5 },
            ),
            (
                semantic::TEXT_COLOR_PLACEHOLDER_DEFAULT,
                if high { 7.0 } else { 4.5 },
            ),
            (
                semantic::TEXT_COLOR_ACCENT_DEFAULT,
                if high { 7.0 } else { 4.5 },
            ),
            (
                semantic::ICON_COLOR_PRIMARY_DEFAULT,
                if high { 7.0 } else { 3.0 },
            ),
            (
                semantic::ICON_COLOR_SECONDARY_DEFAULT,
                if high { 7.0 } else { 3.0 },
            ),
            (
                semantic::BORDER_COLOR_FOCUS_DEFAULT,
                if high { 7.0 } else { 3.0 },
            ),
        ];
        for surface in surfaces {
            let below = design.get(surface);
            for (token, needed) in checks {
                let ratio = design.get(token).contrast_ratio(below);
                if ratio < needed {
                    failures.push(format!(
                        "{key}: {} on {}: {ratio:.2} < {needed}",
                        name(&design, token),
                        name(&design, surface)
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn colored_fills_hold_their_content_in_every_theme() {
    // #60: content on the accent and on the other colored fills, every state.
    let mut design = design();
    let fills = [
        semantic::BACKGROUND_ACCENT_DEFAULT,
        semantic::BACKGROUND_ACCENT_HOVER,
        semantic::BACKGROUND_ACCENT_PRESSED,
        semantic::BACKGROUND_INFO_DEFAULT,
        semantic::BACKGROUND_POSITIVE_DEFAULT,
        semantic::BACKGROUND_NEGATIVE_DEFAULT,
        semantic::BACKGROUND_NOTICE_DEFAULT,
        semantic::BACKGROUND_NEUTRAL_DEFAULT,
        semantic::BACKGROUND_TEXT_SELECTION_DEFAULT,
    ];
    let mut failures = Vec::new();
    for (id, key) in themes(&design) {
        design.set_active_theme(id).unwrap();
        let needed = if key == "high-contrast" { 7.0 } else { 4.5 };
        let on = design.get(semantic::TEXT_COLOR_ON_ACCENT_DEFAULT);
        for fill in fills {
            let content = if fill == semantic::BACKGROUND_TEXT_SELECTION_DEFAULT {
                design.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT)
            } else {
                on
            };
            let ratio = content.contrast_ratio(design.get(fill));
            if ratio < needed {
                failures.push(format!(
                    "{key}: content on {}: {ratio:.2} < {needed}",
                    name(&design, fill)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn outlined_controls_show_their_boundary() {
    // Where a theme outlines its controls, the outline reaches 3:1 against the panel (WCAG
    // 1.4.11), 7:1 in the high-contrast theme. The dark themes without outlines tell controls
    // apart by their fill, which this does not require (`docs/design-system/color.md`).
    let mut design = design();
    for (id, key) in themes(&design) {
        design.set_active_theme(id).unwrap();
        if design.get(semantic::BORDER_WIDTH_CONTROL) <= 0.0 {
            continue;
        }
        let needed = if key == "high-contrast" { 7.0 } else { 3.0 };
        let panel = design.get(semantic::BACKGROUND_RAISED_DEFAULT);
        for border in [
            component::TEXT_FIELD_BORDER_COLOR_DEFAULT,
            component::NUMBER_FIELD_BORDER_COLOR_DEFAULT,
            component::DROPDOWN_BORDER_COLOR_DEFAULT,
            component::CHECKBOX_BORDER_COLOR_DEFAULT,
        ] {
            let ratio = design.get(border).contrast_ratio(panel);
            assert!(
                ratio >= needed,
                "{key}: {}: {ratio:.2} < {needed}",
                name(&design, border)
            );
        }
    }
}

#[test]
fn the_high_contrast_theme_meets_its_own_rules() {
    let mut design = design();
    let id = design.find_theme("high-contrast").unwrap();
    design.set_active_theme(id).unwrap();
    // Outlined controls at 2 pixels, a focus ring of 3.
    assert!(design.get(semantic::BORDER_WIDTH_CONTROL) >= 2.0);
    assert!(design.get(semantic::BORDER_WIDTH_FOCUS) >= 3.0);
    // Disabled content stays visible.
    let base = design.get(semantic::BACKGROUND_BASE_DEFAULT);
    for disabled in [
        semantic::TEXT_COLOR_DISABLED_DEFAULT,
        semantic::ICON_COLOR_DISABLED_DEFAULT,
    ] {
        let ratio = design.get(disabled).contrast_ratio(base);
        assert!(ratio >= 4.5, "{}: {ratio:.2}", name(&design, disabled));
    }
    // Surfaces are black or nearly: nothing but content and borders stands out.
    for surface in [
        semantic::BACKGROUND_BASE_DEFAULT,
        semantic::BACKGROUND_RAISED_DEFAULT,
        semantic::BACKGROUND_APP_FRAME,
    ] {
        assert!(
            design.get(surface).relative_luminance() < 0.02,
            "{}",
            name(&design, surface)
        );
    }
}

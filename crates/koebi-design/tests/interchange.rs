//! Interchange with the Design Tokens format: export, import, strict validation.

#[allow(dead_code, reason = "each test crate uses part of the shared schema")]
mod common;

use common::{component, hex, primitive, semantic};
use koebi_design::dtcg::{export_schema, export_theme, import_theme};
use koebi_design::value::{FontWeight, Value};
use koebi_design::{DesignSystem, Error, Layers, Scope, ThemeId};

static SCHEMA: koebi_design::schema::Schema = common::SCHEMA;

fn design() -> DesignSystem {
    DesignSystem::new(&SCHEMA).expect("the test schema is valid")
}

fn values(design: &DesignSystem, theme: ThemeId, layers: Layers) -> Vec<Value> {
    let view = design.view(theme, layers).expect("live theme");
    SCHEMA
        .tokens()
        .map(|(id, _)| view.value(id).expect("token of the schema"))
        .collect()
}

/// The issues of a failed import, as text.
fn issues(result: Result<ThemeId, Error>) -> String {
    match result {
        Err(Error::Import(issues)) => issues
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
        // Returned as text: the caller's assertions then show what happened.
        other => format!("expected import issues, got {other:?}"),
    }
}

#[test]
fn an_exported_theme_imports_back_to_the_same_values() {
    let mut design = design();
    let green = design.find_theme("green").unwrap();
    let text = export_theme(&design, green).unwrap();
    assert!(text.contains("\"$value\": \"{primitive.color.green.700}\""));
    assert!(text.contains("\"layer\": \"theme\""));

    let copy = text.replace("\"key\": \"green\"", "\"key\": \"green-copy\"");
    let imported = import_theme(&mut design, &copy).unwrap();
    assert_eq!(design.theme(imported).unwrap().name(), "Green");
    assert_eq!(
        values(&design, imported, Layers::THEME),
        values(&design, green, Layers::THEME)
    );
}

#[test]
fn exports_gather_the_theme_its_overrides_and_global_overrides() {
    let mut design = design();
    let light = design.find_theme("light").unwrap();
    design.set_active_theme(light).unwrap();
    design
        .set_value(Scope::ActiveTheme, semantic::RADIUS_CONTROL, 6.0)
        .unwrap();
    design
        .set_alias(Scope::Global, component::BUTTON_BACKGROUND, component::LOGO)
        .unwrap();
    design
        .set_value(Scope::Global, primitive::FONT_FAMILY_SANS, "Inter".into())
        .unwrap();
    design
        .set_value(Scope::Global, primitive::OPACITY_500, 0.25)
        .unwrap();
    let text = export_theme(&design, light).unwrap();
    for expected in [
        "\"layer\": \"theme-override\"",
        "\"layer\": \"global-override\"",
        "\"{component.logo}\"",
        "\"Inter\"",
    ] {
        assert!(text.contains(expected), "missing {expected} in\n{text}");
    }

    // Imported as a theme, the file reproduces everything that differs from the base theme.
    let copy = text.replace("\"key\": \"light\"", "\"key\": \"light-custom\"");
    let imported = import_theme(&mut design, &copy).unwrap();
    assert_eq!(
        values(&design, imported, Layers::THEME),
        values(&design, light, Layers::DEFINITIONS)
    );
}

#[test]
fn typography_composites_are_exported_and_imported_whole() {
    let mut design = design();
    design
        .set_value(Scope::ActiveTheme, semantic::TEXT_BODY.font_size, 17.0)
        .unwrap();
    let text = export_theme(&design, ThemeId::BASE).unwrap();
    assert!(text.contains("\"$type\": \"typography\""));
    assert!(text.contains("\"fontSize\": {"));
    assert!(text.contains("\"fontWeight\": \"{semantic.font.weight.body}\""));

    let copy = text.replace("\"key\": \"dark\"", "\"key\": \"bigger-body\"");
    let imported = import_theme(&mut design, &copy).unwrap();
    let style = design
        .view(imported, Layers::THEME)
        .unwrap()
        .typography(semantic::TEXT_BODY);
    assert_eq!(style.font_size, 17.0);
    assert_eq!(style.font_weight, FontWeight::REGULAR);
}

#[test]
fn the_schema_export_documents_every_token() {
    let text = export_schema(&design());
    for expected in [
        "\"primitive\": {",
        "\"$type\": \"color\"",
        "\"hex\": \"#5d89ff\"",
        "\"$value\": \"{primitive.color.blue.700}\"",
        "\"$description\": \"Background of accent elements (semantic to semantic).\"",
        "\"tier\": \"component\"",
        "\"kind\": \"integer\"",
        "\"range\": {",
        "\"unit\": \"ms\"",
        "\"$type\": \"typography\"",
        "\"scrollbar\": {",
    ] {
        assert!(text.contains(expected), "missing {expected}");
    }
    // Composite fields appear inside their composite only.
    assert!(!text.contains("font-size"));
}

#[test]
fn colors_import_from_every_supported_notation() {
    let mut design = design();
    let document = r##"{
      "$extensions": { "io.github.kolosta.koebi": { "theme": { "key": "colors", "name": "Colors" } } },
      "primitive": { "color": {
        "blue": { "700": { "$type": "color", "$value": "#336699" } },
        "green": { "700": { "$value": { "colorSpace": "srgb", "components": [0, 0.5, 1], "alpha": 0.5 } } },
        "red": { "700": { "$value": { "colorSpace": "hsl", "components": [0, 100, 50] } } },
        "gray": {
          "50": { "$value": { "colorSpace": "oklch", "components": [1, 0, 0] } },
          "500": { "$value": { "colorSpace": "srgb-linear", "components": [0.2158605, 0.2158605, 0.2158605] } }
        }
      } }
    }"##;
    let theme = import_theme(&mut design, document).unwrap();
    let view = design.view(theme, Layers::THEME).unwrap();
    assert_eq!(view.get(primitive::COLOR_BLUE_700), hex(0x33_66_99));
    assert_eq!(
        view.get(primitive::COLOR_GREEN_700).to_rgba8(),
        [0, 128, 255, 128]
    );
    assert_eq!(
        view.get(primitive::COLOR_RED_700).to_rgba8(),
        [255, 0, 0, 255]
    );
    assert_eq!(
        view.get(primitive::COLOR_GRAY_50).to_rgba8(),
        [255, 255, 255, 255]
    );
    assert_eq!(
        view.get(primitive::COLOR_GRAY_500).to_rgba8(),
        [128, 128, 128, 255]
    );
}

#[test]
fn every_problem_is_reported_with_its_location_and_nothing_is_created() {
    let mut design = design();
    let count = design.themes().count();
    let document = r##"{
  "$extensions": { "io.github.kolosta.koebi": { "theme": { "key": "broken", "name": "Broken" } } },
  "$unknown": 1,
  "primitive": {
    "radius": { "100": { "$type": "dimension", "$value": { "value": 99, "unit": "px" } } },
    "opacity": { "500": { "$value": "half" } },
    "missing": { "$value": 1 }
  },
  "component": {
    "button": {
      "background": { "$value": "{primitive.color.red.700}" },
      "radius": { "$type": "color", "$value": "#ffffff" },
      "label": { "$value": "{semantic.nowhere}" }
    }
  },
  "styles": {}
}"##;
    let report = issues(import_theme(&mut design, document));
    for expected in [
        "line 3, column 15: unknown property $unknown",
        "primitive.radius.100: 99 is outside the range",
        "primitive.opacity.500: expected a number, found a string",
        "primitive.missing: unknown token",
        "component.button.background: component.button.background cannot alias primitive.color.red.700",
        "component.button.radius: component.button.radius holds a dimension, not a color",
        "component.button.label: unknown alias target {semantic.nowhere}",
        "styles: unknown tier",
    ] {
        assert!(
            report.contains(expected),
            "missing {expected:?} in:\n{report}"
        );
    }
    assert_eq!(design.themes().count(), count);
}

#[test]
fn files_without_identity_or_with_bad_syntax_are_refused() {
    let mut design = design();
    let report = issues(import_theme(&mut design, "{ \"primitive\": {} }"));
    assert!(report.contains("missing theme identity"), "{report}");
    let report = issues(import_theme(&mut design, "{\n  \"a\": 1,\n}"));
    assert!(report.starts_with("line 3, column 1:"), "{report}");
    let report = issues(import_theme(
        &mut design,
        r#"{ "$extensions": { "io.github.kolosta.koebi": { "theme": { "key": "light", "name": "Again" } } } }"#,
    ));
    assert!(report.contains("already has the key"), "{report}");
}

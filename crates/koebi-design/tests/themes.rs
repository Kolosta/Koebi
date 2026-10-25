//! Themes: switching, previewing other themes, creating and removing themes at run time.

#[allow(dead_code, reason = "each test crate uses part of the shared schema")]
mod common;

use common::{component, hex, primitive, semantic};
use koebi_design::value::{Definition, Value};
use koebi_design::{DesignSystem, Error, Layers, NewTheme, Origin, ThemeId, ThemeOrigin};

static SCHEMA: koebi_design::schema::Schema = common::SCHEMA;

fn design() -> DesignSystem {
    DesignSystem::new(&SCHEMA).expect("the test schema is valid")
}

#[test]
fn themes_of_the_schema_are_listed_base_first() {
    let design = design();
    let themes: Vec<_> = design
        .themes()
        .map(|(id, theme)| (id, theme.key().to_owned(), theme.origin()))
        .collect();
    assert_eq!(
        themes[0],
        (ThemeId::BASE, "dark".to_owned(), ThemeOrigin::Base)
    );
    assert_eq!(themes[1].1, "light");
    assert_eq!(
        themes[2],
        (themes[2].0, "green".to_owned(), ThemeOrigin::BuiltIn)
    );
    assert_eq!(design.active_theme(), ThemeId::BASE);
    assert_eq!(design.theme(ThemeId::BASE).unwrap().name(), "Dark");
}

#[test]
fn a_theme_only_replaces_what_it_defines() {
    let mut design = design();
    let light = design.find_theme("light").unwrap();
    design.set_active_theme(light).unwrap();
    assert_eq!(design.get(semantic::BACKGROUND_BASE), hex(0xF8_F8_F8));
    assert_eq!(design.get(component::BUTTON_LABEL), hex(0x13_13_13));
    // Not defined by the light theme: base values.
    assert_eq!(design.get(component::BUTTON_BACKGROUND), hex(0x5D_89_FF));
    assert_eq!(design.origin(semantic::BACKGROUND_BASE), Origin::Theme);
    assert_eq!(design.origin(semantic::BACKGROUND_ACCENT), Origin::Base);
}

#[test]
fn theme_definitions_propagate_to_every_dependent() {
    let mut design = design();
    let green = design.find_theme("green").unwrap();
    design.set_active_theme(green).unwrap();
    for color in [
        design.get(semantic::ACCENT_COLOR_700),
        design.get(semantic::BACKGROUND_ACCENT),
        design.get(component::BUTTON_BACKGROUND),
        design.get(component::BUTTON_BACKGROUND_HOVER),
    ] {
        assert_eq!(color, hex(0x13_9E_5C));
    }
    // Switching back restores the base values.
    design.set_active_theme(ThemeId::BASE).unwrap();
    assert_eq!(
        design.get(component::BUTTON_BACKGROUND_HOVER),
        hex(0x5D_89_FF)
    );
}

#[test]
fn other_themes_are_readable_without_switching() {
    let design = design();
    let (light, green) = (
        design.find_theme("light").unwrap(),
        design.find_theme("green").unwrap(),
    );
    // Preview cards read other themes while the active theme stays the same.
    let light_view = design.view(light, Layers::ALL).unwrap();
    let green_view = design.view(green, Layers::ALL).unwrap();
    assert_eq!(light_view.get(semantic::BACKGROUND_BASE), hex(0xF8_F8_F8));
    assert_eq!(
        green_view.get(component::BUTTON_BACKGROUND),
        hex(0x13_9E_5C)
    );
    assert_eq!(design.get(component::BUTTON_BACKGROUND), hex(0x5D_89_FF));
    assert_eq!(design.active_theme(), ThemeId::BASE);
}

#[test]
fn themes_are_created_at_run_time_with_values_and_aliases() {
    let mut design = design();
    let revision = design.revision();
    let theme = NewTheme::new("user.red", "Red")
        .with(
            semantic::ACCENT_COLOR_700,
            Definition::Alias(primitive::COLOR_RED_700.id()),
        )
        .with(
            semantic::RADIUS_CONTROL,
            Definition::Value(Value::Dimension(8.0)),
        );
    let red = design.create_theme(theme).unwrap();
    assert!(design.revision() > revision);
    assert_eq!(design.theme(red).unwrap().origin(), ThemeOrigin::User);
    assert_eq!(design.theme_definitions(red).unwrap().count(), 2);

    design.set_active_theme(red).unwrap();
    assert_eq!(design.get(component::BUTTON_BACKGROUND), hex(0xE3_40_3C));
    assert_eq!(design.get(component::BUTTON_RADIUS), 8.0);
}

#[test]
fn invalid_themes_are_rejected_without_side_effects() {
    let mut design = design();
    let count = design.themes().count();
    let cases = [
        (NewTheme::new("light", "Again"), "duplicate key"),
        (NewTheme::new("Not A Key", "Bad"), "invalid key"),
        (
            NewTheme::new("tiers", "Tiers").with(
                component::BUTTON_BACKGROUND,
                Definition::Alias(primitive::COLOR_RED_700.id()),
            ),
            "component aliasing a primitive",
        ),
        (
            NewTheme::new("kinds", "Kinds").with(
                semantic::RADIUS_CONTROL,
                Definition::Value(Value::Number(2.0)),
            ),
            "number for a dimension",
        ),
        (
            NewTheme::new("range", "Range").with(
                semantic::RADIUS_CONTROL,
                Definition::Value(Value::Dimension(64.0)),
            ),
            "radius above its range",
        ),
        (
            NewTheme::new("twice", "Twice")
                .with(semantic::CHAIN_1, Definition::Value(Value::Color(hex(0))))
                .with(semantic::CHAIN_1, Definition::Value(Value::Color(hex(1)))),
            "token defined twice",
        ),
        (
            NewTheme::new("loop", "Loop")
                .with(semantic::CHAIN_1, Definition::Alias(semantic::CHAIN_8.id())),
            "alias cycle",
        ),
    ];
    for (theme, case) in cases {
        assert!(
            design.create_theme(theme).is_err(),
            "{case} must be rejected"
        );
    }
    assert_eq!(design.themes().count(), count);
    assert_eq!(design.revision(), 0);
}

#[test]
fn only_inactive_run_time_themes_can_be_removed() {
    let mut design = design();
    let light = design.find_theme("light").unwrap();
    assert!(matches!(
        design.remove_theme(light),
        Err(Error::BuiltInTheme(_))
    ));
    assert!(matches!(
        design.remove_theme(ThemeId::BASE),
        Err(Error::BuiltInTheme(_))
    ));

    let user = design.create_theme(NewTheme::new("user", "User")).unwrap();
    design.set_active_theme(user).unwrap();
    assert!(matches!(
        design.remove_theme(user),
        Err(Error::ActiveTheme(_))
    ));
    design.set_active_theme(light).unwrap();
    design.remove_theme(user).unwrap();
    assert!(design.view(user, Layers::ALL).is_none());
    assert!(matches!(
        design.set_active_theme(user),
        Err(Error::UnknownTheme(_))
    ));
    // The key is free again.
    design.create_theme(NewTheme::new("user", "User")).unwrap();
}

#[test]
fn switching_to_the_active_theme_changes_nothing() {
    let mut design = design();
    design.set_active_theme(ThemeId::BASE).unwrap();
    assert_eq!(design.revision(), 0);
}

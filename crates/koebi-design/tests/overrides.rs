//! Overrides: per theme and global, values and aliases, validation, cycles and views.

#[allow(dead_code, reason = "each test crate uses part of the shared schema")]
mod common;

use common::{component, hex, primitive, semantic};
use koebi_design::value::{Definition, Value};
use koebi_design::{DesignSystem, Error, Layers, NewTheme, Origin, Scope, ThemeId};

static SCHEMA: koebi_design::schema::Schema = common::SCHEMA;

const WHITE: u32 = 0xFF_FF_FF;
const BLACK: u32 = 0x00_00_00;

fn design() -> DesignSystem {
    DesignSystem::new(&SCHEMA).expect("the test schema is valid")
}

fn theme(design: &DesignSystem, key: &str) -> ThemeId {
    design.find_theme(key).expect("theme of the test schema")
}

#[test]
fn a_global_override_of_a_primitive_reaches_every_dependent_in_every_theme() {
    let mut design = design();
    design
        .set_value(Scope::Global, primitive::COLOR_RED_700, hex(WHITE))
        .unwrap();
    // The whole chain: 8 semantic tokens and a component token.
    assert_eq!(design.get(semantic::CHAIN_1), hex(WHITE));
    assert_eq!(design.get(component::DEEP), hex(WHITE));
    let light = theme(&design, "light");
    design.set_active_theme(light).unwrap();
    assert_eq!(design.get(component::DEEP), hex(WHITE));
}

#[test]
fn overrides_of_semantic_and_component_tokens_reach_their_dependents() {
    let mut design = design();
    design
        .set_value(Scope::ActiveTheme, semantic::BACKGROUND_ACCENT, hex(WHITE))
        .unwrap();
    assert_eq!(design.get(component::BUTTON_BACKGROUND), hex(WHITE));
    assert_eq!(design.get(component::BUTTON_BACKGROUND_HOVER), hex(WHITE));
    // Upstream tokens keep their value.
    assert_eq!(design.get(semantic::ACCENT_COLOR_700), hex(0x5D_89_FF));

    design
        .set_value(Scope::ActiveTheme, component::BUTTON_BACKGROUND, hex(BLACK))
        .unwrap();
    assert_eq!(design.get(component::BUTTON_BACKGROUND_HOVER), hex(BLACK));
    assert_eq!(design.get(semantic::BACKGROUND_ACCENT), hex(WHITE));
}

#[test]
fn theme_overrides_stay_in_their_theme() {
    let mut design = design();
    let light = theme(&design, "light");
    // Overriding an inactive theme does not change what is shown...
    design
        .set_value(Scope::Theme(light), semantic::BACKGROUND_BASE, hex(WHITE))
        .unwrap();
    assert_eq!(design.get(semantic::BACKGROUND_BASE), hex(0x13_13_13));
    // ...until that theme is active.
    design.set_active_theme(light).unwrap();
    assert_eq!(design.get(semantic::BACKGROUND_BASE), hex(WHITE));
    // `ActiveTheme` means the theme active when the override is set.
    design
        .set_value(Scope::ActiveTheme, semantic::TEXT_PRIMARY, hex(BLACK))
        .unwrap();
    design.set_active_theme(ThemeId::BASE).unwrap();
    assert_eq!(design.get(semantic::TEXT_PRIMARY), hex(0xF8_F8_F8));
    assert!(
        design
            .override_of(Scope::Theme(light), semantic::TEXT_PRIMARY)
            .is_some()
    );
}

#[test]
fn layers_stack_in_order_and_views_select_them() {
    let mut design = design();
    let light = theme(&design, "light");
    design.set_active_theme(light).unwrap();
    let token = semantic::BACKGROUND_BASE;
    design
        .set_value(Scope::ActiveTheme, token, hex(0x11_11_11))
        .unwrap();
    design
        .set_value(Scope::Global, token, hex(0x22_22_22))
        .unwrap();

    // Global overrides win over theme overrides, which win over the theme.
    assert_eq!(design.get(token), hex(0x22_22_22));
    assert_eq!(design.origin(token), Origin::GlobalOverride);

    let read = |layers: Layers| design.view(light, layers).unwrap().get(token);
    let origin = |layers: Layers| design.view(light, layers).unwrap().origin(token);
    assert_eq!(read(Layers::NONE), hex(0x13_13_13), "base theme");
    assert_eq!(
        read(Layers::THEME),
        hex(0xF8_F8_F8),
        "the theme as designed"
    );
    assert_eq!(
        read(Layers::THEME | Layers::THEME_OVERRIDES),
        hex(0x11_11_11)
    );
    assert_eq!(
        read(Layers::THEME | Layers::GLOBAL_OVERRIDES),
        hex(0x22_22_22)
    );
    assert_eq!(
        read(Layers::GLOBAL_OVERRIDES),
        hex(0x22_22_22),
        "global only"
    );
    assert_eq!(origin(Layers::NONE), Origin::Base);
    assert_eq!(origin(Layers::THEME), Origin::Theme);
    assert_eq!(
        origin(Layers::THEME | Layers::THEME_OVERRIDES),
        Origin::ThemeOverride
    );

    // Removing the global override reveals the theme override.
    assert!(design.reset(Scope::Global, token).unwrap());
    assert_eq!(design.get(token), hex(0x11_11_11));
    assert!(!design.reset(Scope::Global, token).unwrap());
    design.reset_scope(Scope::ActiveTheme).unwrap();
    assert_eq!(design.get(token), hex(0xF8_F8_F8));
}

#[test]
fn views_on_other_themes_include_their_overrides() {
    let mut design = design();
    let green = theme(&design, "green");
    design
        .set_value(Scope::Theme(green), primitive::COLOR_GREEN_700, hex(WHITE))
        .unwrap();
    let preview = design.view(green, Layers::ALL).unwrap();
    assert_eq!(preview.get(component::BUTTON_BACKGROUND), hex(WHITE));
    let original = design.view(green, Layers::THEME).unwrap();
    assert_eq!(original.get(component::BUTTON_BACKGROUND), hex(0x13_9E_5C));
}

#[test]
fn alias_overrides_repoint_tokens_and_keep_following_them() {
    let mut design = design();
    design
        .set_alias(
            Scope::Global,
            semantic::ACCENT_COLOR_700,
            primitive::COLOR_RED_700,
        )
        .unwrap();
    assert_eq!(design.get(component::BUTTON_BACKGROUND), hex(0xE3_40_3C));
    // The new target is followed: overriding it reaches the re-pointed chain.
    design
        .set_value(Scope::Global, primitive::COLOR_RED_700, hex(WHITE))
        .unwrap();
    assert_eq!(design.get(component::BUTTON_BACKGROUND), hex(WHITE));
    assert_eq!(design.get(component::DEEP), hex(WHITE));
    // Component to component alias override.
    design
        .set_alias(
            Scope::Global,
            component::BUTTON_BACKGROUND_HOVER,
            component::LOGO,
        )
        .unwrap();
    assert_eq!(
        design.get(component::BUTTON_BACKGROUND_HOVER),
        hex(0x9B_59_F0)
    );
}

#[test]
fn untyped_overrides_are_validated_at_run_time() {
    let mut design = design();
    let set = |design: &mut DesignSystem, token, definition| {
        design.set_override(Scope::Global, token, definition)
    };
    let tier = set(
        &mut design,
        component::BUTTON_BACKGROUND.id(),
        Definition::Alias(primitive::COLOR_RED_700.id()),
    );
    assert!(matches!(tier, Err(Error::TierViolation { .. })));
    let kind = set(
        &mut design,
        semantic::RADIUS_CONTROL.id(),
        Definition::Alias(primitive::OPACITY_500.id()),
    );
    assert!(matches!(kind, Err(Error::KindMismatch { .. })));
    let value_kind = set(
        &mut design,
        semantic::RADIUS_CONTROL.id(),
        Definition::Value(Value::Color(hex(WHITE))),
    );
    assert!(matches!(value_kind, Err(Error::KindMismatch { .. })));
    let range = set(
        &mut design,
        component::BUTTON_RADIUS.id(),
        Definition::Value(Value::Dimension(-1.0)),
    );
    let Err(Error::OutOfRange { range, .. }) = range else {
        panic!("expected an out-of-range error, got {range:?}");
    };
    assert_eq!((range.min(), range.max()), (0.0, 32.0));
    let nan = set(
        &mut design,
        primitive::OPACITY_500.id(),
        Definition::Value(Value::Number(f32::NAN)),
    );
    assert!(matches!(nan, Err(Error::InvalidValue { .. })));
    let color = design.set_value(
        Scope::Global,
        primitive::COLOR_BLUE_700,
        koebi_core::Color::new(1.5, 0.0, 0.0, 1.0),
    );
    assert!(matches!(color, Err(Error::InvalidValue { .. })));
    // Nothing invalid was applied.
    assert!(design.overrides(Scope::Global).unwrap().next().is_none());
}

#[test]
fn overrides_of_removed_themes_are_rejected() {
    let mut design = design();
    let user = design.create_theme(NewTheme::new("user", "User")).unwrap();
    design.remove_theme(user).unwrap();
    let result = design.set_value(Scope::Theme(user), primitive::RADIUS_100, 4.0);
    assert!(matches!(result, Err(Error::UnknownTheme(_))));
    assert!(
        design
            .override_of(Scope::Theme(user), primitive::RADIUS_100)
            .is_none()
    );
}

#[test]
fn alias_cycles_are_rejected_in_every_layer_combination() {
    let mut design = design();
    // A direct cycle through the chain.
    let direct = design.set_alias(Scope::Global, semantic::CHAIN_1, semantic::CHAIN_8);
    assert!(matches!(direct, Err(Error::Cycle { .. })));
    // Self alias.
    let own = design.set_alias(Scope::Global, semantic::CHAIN_3, semantic::CHAIN_3);
    assert!(matches!(own, Err(Error::Cycle { .. })));
    // Component cycle.
    design
        .set_alias(Scope::Global, component::BUTTON_BACKGROUND, component::DEEP)
        .unwrap();
    let back = design.set_alias(
        Scope::ActiveTheme,
        component::DEEP,
        component::BUTTON_BACKGROUND_HOVER,
    );
    assert!(matches!(back, Err(Error::Cycle { .. })));
    // Nothing was applied.
    assert_eq!(design.get(component::DEEP), hex(0xE3_40_3C));
}

#[test]
fn global_aliases_are_checked_against_every_theme() {
    let mut design = design();
    // In a run-time theme, CHAIN_1 aliases ACCENT_COLOR_700.
    design
        .create_theme(NewTheme::new("chained", "Chained").with(
            semantic::CHAIN_1,
            Definition::Alias(semantic::ACCENT_COLOR_700.id()),
        ))
        .unwrap();
    // Harmless in the base theme, but a cycle in the "chained" theme.
    let result = design.set_alias(Scope::Global, semantic::ACCENT_COLOR_700, semantic::CHAIN_5);
    assert!(matches!(result, Err(Error::Cycle { .. })));
}

#[test]
fn a_removal_that_would_reveal_a_cycle_is_rejected() {
    let mut design = design();
    // CHAIN_2 aliases CHAIN_1 in the base theme. Give CHAIN_2 a value, then make CHAIN_1
    // alias CHAIN_2: fine while the value hides CHAIN_2's base alias.
    design
        .set_value(Scope::ActiveTheme, semantic::CHAIN_2, hex(WHITE))
        .unwrap();
    design
        .set_alias(Scope::ActiveTheme, semantic::CHAIN_1, semantic::CHAIN_2)
        .unwrap();
    assert_eq!(design.get(component::DEEP), hex(WHITE));
    // Removing the value would reveal CHAIN_2 -> CHAIN_1 -> CHAIN_2.
    let result = design.reset(Scope::ActiveTheme, semantic::CHAIN_2);
    assert!(matches!(result, Err(Error::Cycle { .. })));
    assert_eq!(design.get(component::DEEP), hex(WHITE));
    // Removing the alias first makes the removal possible.
    assert!(design.reset(Scope::ActiveTheme, semantic::CHAIN_1).unwrap());
    assert!(design.reset(Scope::ActiveTheme, semantic::CHAIN_2).unwrap());
    assert_eq!(design.get(component::DEEP), hex(0xE3_40_3C));
}

#[test]
fn every_change_increments_the_revision() {
    let mut design = design();
    let mut last = design.revision();
    let mut step = |design: &DesignSystem| {
        assert!(design.revision() > last);
        last = design.revision();
    };
    design
        .set_value(Scope::Global, primitive::RADIUS_100, 3.0)
        .unwrap();
    step(&design);
    design
        .set_alias(
            Scope::Global,
            semantic::RADIUS_CONTROL,
            primitive::RADIUS_200,
        )
        .unwrap();
    step(&design);
    design
        .reset(Scope::Global, semantic::RADIUS_CONTROL)
        .unwrap();
    step(&design);
    design.reset_scope(Scope::Global).unwrap();
    step(&design);
    assert_eq!(design.get(component::BUTTON_RADIUS), 2.0);
}

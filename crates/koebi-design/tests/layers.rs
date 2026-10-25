//! Per-token layer masks, color vision and incremental propagation.

#[allow(dead_code, reason = "each test crate uses part of the shared schema")]
mod common;

use common::{component, hex, primitive, semantic};
use koebi_design::{ColorVision, Deficiency, DesignSystem, Error, Layers, Scope, ThemeId};

static SCHEMA: koebi_design::schema::Schema = common::SCHEMA;

const WHITE: u32 = 0xFF_FF_FF;

fn design() -> DesignSystem {
    DesignSystem::new(&SCHEMA).expect("the test schema is valid")
}

fn deutan() -> ColorVision {
    ColorVision::Correct {
        deficiency: Deficiency::Deutan,
        strength: 1.0,
    }
}

#[test]
fn a_token_can_ignore_the_overrides() {
    let mut design = design();
    design
        .set_token_layers(component::BUTTON_LABEL, Layers::ALL - Layers::OVERRIDES)
        .unwrap();
    design
        .set_value(Scope::Global, semantic::TEXT_PRIMARY, hex(WHITE))
        .unwrap();
    // Everything else follows the override; the opted-out token keeps its theme value,
    // through its whole chain of aliases.
    assert_eq!(design.get(semantic::TEXT_PRIMARY), hex(WHITE));
    assert_eq!(design.get(component::BUTTON_LABEL), hex(0xF8_F8_F8));
    // It still follows themes.
    let light = design.find_theme("light").unwrap();
    design.set_active_theme(light).unwrap();
    assert_eq!(design.get(component::BUTTON_LABEL), hex(0x13_13_13));
}

#[test]
fn a_token_can_stay_on_the_base_theme() {
    let mut design = design();
    design
        .set_token_layers(component::BUTTON_BACKGROUND, Layers::NONE)
        .unwrap();
    let green = design.find_theme("green").unwrap();
    design.set_active_theme(green).unwrap();
    design
        .set_value(Scope::Global, primitive::COLOR_GREEN_700, hex(WHITE))
        .unwrap();
    assert_eq!(design.get(component::BUTTON_BACKGROUND), hex(0x5D_89_FF));
    // Its dependents inherit the mask through the alias.
    assert_eq!(
        design.get(component::BUTTON_BACKGROUND_HOVER),
        hex(0x5D_89_FF)
    );
    assert_eq!(design.get(semantic::BACKGROUND_ACCENT), hex(WHITE));
    assert_eq!(
        design.token_layers(component::BUTTON_BACKGROUND),
        Layers::NONE
    );
}

#[test]
fn masks_apply_to_the_masked_token_and_what_it_aliases() {
    let mut design = design();
    // A mask on a primitive applies to every token that aliases it.
    design
        .set_token_layers(
            primitive::COLOR_RED_700,
            Layers::ALL - Layers::GLOBAL_OVERRIDES,
        )
        .unwrap();
    design
        .set_value(Scope::Global, primitive::COLOR_RED_700, hex(WHITE))
        .unwrap();
    assert_eq!(design.get(component::DEEP), hex(0xE3_40_3C));
    // A global override higher in the chain is not masked.
    design
        .set_value(Scope::Global, semantic::CHAIN_4, hex(WHITE))
        .unwrap();
    assert_eq!(design.get(component::DEEP), hex(WHITE));
    assert_eq!(design.get(semantic::CHAIN_3), hex(0xE3_40_3C));
}

#[test]
fn color_vision_transforms_colors_only() {
    let mut design = design();
    let accent = design.get(component::BUTTON_BACKGROUND);
    design.set_color_vision(deutan()).unwrap();
    let corrected = design.get(component::BUTTON_BACKGROUND);
    assert_eq!(corrected, deutan().matrix().apply(accent));
    assert_ne!(corrected, accent);
    // Grays are unchanged, other kinds untouched.
    let gray = design.get(semantic::BORDER_NEUTRAL);
    assert!((gray.r - gray.g).abs() < 2e-3 && (gray.g - gray.b).abs() < 2e-3);
    assert_eq!(design.get(component::BUTTON_RADIUS), 2.0);
    // Views without the color vision layer give the original colors.
    let original = design.view(ThemeId::BASE, Layers::DEFINITIONS).unwrap();
    assert_eq!(original.get(component::BUTTON_BACKGROUND), accent);
    // Back to normal vision.
    design.set_color_vision(ColorVision::Normal).unwrap();
    assert_eq!(design.get(component::BUTTON_BACKGROUND), accent);
}

#[test]
fn exempt_colors_are_never_transformed() {
    let mut design = design();
    let brand = design.get(primitive::COLOR_PURPLE_500);
    // Exempting the primitive keeps it true wherever it is used: semantic and component
    // tokens aliasing it are not transformed either.
    design
        .set_token_layers(
            primitive::COLOR_PURPLE_500,
            Layers::ALL - Layers::COLOR_VISION,
        )
        .unwrap();
    design.set_color_vision(deutan()).unwrap();
    assert_eq!(design.get(semantic::BRAND_LOGO), brand);
    assert_eq!(design.get(component::LOGO), brand);
    assert_ne!(design.get(component::BUTTON_BACKGROUND), hex(0x5D_89_FF));
}

#[test]
fn a_component_property_can_be_exempt_alone() {
    let mut design = design();
    let brand = design.get(primitive::COLOR_PURPLE_500);
    design
        .set_token_layers(component::LOGO, Layers::ALL - Layers::COLOR_VISION)
        .unwrap();
    design.set_color_vision(deutan()).unwrap();
    assert_eq!(design.get(component::LOGO), brand);
    // The semantic token it aliases is still transformed for other uses.
    assert_eq!(
        design.get(semantic::BRAND_LOGO),
        deutan().matrix().apply(brand)
    );
}

#[test]
fn invalid_color_vision_settings_change_nothing() {
    let mut design = design();
    let result = design.set_color_vision(ColorVision::Simulate {
        deficiency: Deficiency::Protan,
        severity: 2.0,
    });
    assert!(matches!(result, Err(Error::InvalidColorVision(_))));
    assert_eq!(design.color_vision(), ColorVision::Normal);
    assert_eq!(design.revision(), 0);
}

#[test]
fn changes_only_resolve_the_tokens_that_depend_on_them() {
    let mut design = design();
    // Changing a primitive used by one long chain: the primitive, 8 chain links, DEEP.
    design
        .set_value(Scope::Global, primitive::COLOR_RED_700, hex(WHITE))
        .unwrap();
    assert_eq!(design.last_refresh_len(), 10);
    // Changing a leaf: only itself.
    design
        .set_value(Scope::Global, component::DEEP, hex(WHITE))
        .unwrap();
    assert_eq!(design.last_refresh_len(), 1);
    // The accent: ACCENT_COLOR_700, BACKGROUND_ACCENT, BUTTON_BACKGROUND, its hover state.
    design
        .set_value(Scope::ActiveTheme, semantic::ACCENT_COLOR_700, hex(WHITE))
        .unwrap();
    assert_eq!(design.last_refresh_len(), 4);
    // After an alias re-points a token, the new dependency is followed too.
    design
        .set_alias(
            Scope::Global,
            semantic::BRAND_LOGO,
            primitive::COLOR_BLUE_700,
        )
        .unwrap();
    design
        .set_value(Scope::Global, primitive::COLOR_BLUE_700, hex(0x00_00_FF))
        .unwrap();
    assert_eq!(design.get(component::LOGO), hex(0x00_00_FF));
}

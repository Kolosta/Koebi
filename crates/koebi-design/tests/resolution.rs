//! Resolution of every kind of reference in the base theme.

#[allow(dead_code, reason = "each test crate uses part of the shared schema")]
mod common;

use core::time::Duration;

use common::{component, hex, primitive, semantic};
use koebi_design::kind::{self, Kind};
use koebi_design::value::{CubicBezier, FontWeight, Value};
use koebi_design::{DesignSystem, Layers, Origin, Resolve, Scope, TokenId};

static SCHEMA: koebi_design::schema::Schema = common::SCHEMA;

fn design() -> DesignSystem {
    DesignSystem::new(&SCHEMA).expect("the test schema is valid")
}

#[test]
fn schema_passes_the_run_time_validation() {
    assert_eq!(SCHEMA.validate(), Ok(()));
    assert_eq!(SCHEMA.len(), 21 + 16 + 5 + 8);
}

#[test]
fn primitives_hold_values_of_every_kind() {
    let design = design();
    assert_eq!(design.get(primitive::COLOR_BLUE_700), hex(0x5D_89_FF));
    assert_eq!(design.get(primitive::RADIUS_100), 2.0);
    assert_eq!(design.get(primitive::OPACITY_500), 0.5);
    assert_eq!(design.get(primitive::LAYER_MID), 100);
    assert_eq!(
        design.get(primitive::DURATION_FAST),
        Duration::from_millis(100)
    );
    assert_eq!(
        design.get(primitive::EASING_EASE),
        CubicBezier::new(0.25, 0.1, 0.25, 1.0)
    );
    assert_eq!(design.get(primitive::FONT_WEIGHT_BOLD), FontWeight::BOLD);
    assert_eq!(&*design.get(primitive::FONT_FAMILY_SANS), "Noto Sans");
}

#[test]
fn semantic_tokens_alias_primitives_and_semantics() {
    let design = design();
    // semantic -> primitive
    assert_eq!(design.get(semantic::ACCENT_COLOR_700), hex(0x5D_89_FF));
    // semantic -> semantic -> primitive
    assert_eq!(design.get(semantic::BACKGROUND_ACCENT), hex(0x5D_89_FF));
    assert_eq!(design.get(semantic::RADIUS_CONTROL), 2.0);
}

#[test]
fn component_tokens_alias_semantics_and_components() {
    let design = design();
    // component -> semantic -> semantic -> primitive
    assert_eq!(design.get(component::BUTTON_BACKGROUND), hex(0x5D_89_FF));
    // component -> component -> semantic -> ...
    assert_eq!(
        design.get(component::BUTTON_BACKGROUND_HOVER),
        hex(0x5D_89_FF)
    );
    assert_eq!(design.get(component::BUTTON_RADIUS), 2.0);
}

#[test]
fn component_tokens_may_hold_their_own_value() {
    let mut design = design();
    assert_eq!(design.get(component::SCROLLBAR_SIZE), 14.0);
    let range = design
        .range(component::SCROLLBAR_SIZE)
        .expect("declared range");
    assert_eq!((range.min(), range.max()), (1.0, 32.0));
    design
        .set_value(Scope::Global, component::SCROLLBAR_SIZE, 10.0)
        .unwrap();
    assert_eq!(design.get(component::SCROLLBAR_SIZE), 10.0);
    let too_big = design.set_value(Scope::Global, component::SCROLLBAR_SIZE, 40.0);
    assert!(too_big.is_err());
}

#[test]
fn long_chains_resolve_to_their_primitive() {
    let design = design();
    // component -> 8 semantic tokens -> primitive
    assert_eq!(design.get(component::DEEP), hex(0xE3_40_3C));
    assert_eq!(design.get(semantic::CHAIN_4), hex(0xE3_40_3C));
}

#[test]
fn typography_composites_resolve_each_field() {
    let design = design();
    let body = design.typography(semantic::TEXT_BODY);
    assert_eq!(&*body.font_family, "Noto Sans");
    assert_eq!(body.font_size, 15.0);
    assert_eq!(body.font_weight, FontWeight::REGULAR);
    assert_eq!(body.line_height, 1.5);
    assert_eq!(body.letter_spacing, 0.0);
    // Fields are tokens with their own paths.
    let size = design.info(semantic::TEXT_BODY.font_size).unwrap();
    assert_eq!(size.qualified().to_string(), "semantic.text.body.font-size");
    assert_eq!(size.kind(), kind::Dimension::ID);
}

#[test]
fn tokens_are_found_by_path_and_described() {
    let design = design();
    let id = design.find("component.button.background-hover").unwrap();
    assert_eq!(id, TokenId::from(component::BUTTON_BACKGROUND_HOVER));
    assert_eq!(design.find("component.button.missing"), None);
    assert_eq!(design.find("nonsense"), None);
    let info = design.info(component::BUTTON_BACKGROUND_HOVER).unwrap();
    assert_eq!(info.ident(), "BUTTON_BACKGROUND_HOVER");
    assert_eq!(
        info.description(),
        "Hovered button background (component to component)."
    );
    assert_eq!(design.value(id), Some(&Value::Color(hex(0x5D_89_FF))));
}

#[test]
fn ranges_are_inherited_through_aliases() {
    let design = design();
    let own = design.range(primitive::RADIUS_100).unwrap();
    assert_eq!((own.min(), own.max()), (0.0, 32.0));
    // component -> semantic -> primitive: the primitive's range applies.
    assert_eq!(design.range(component::BUTTON_RADIUS), Some(own));
    assert_eq!(design.range(component::BUTTON_BACKGROUND), None);
}

#[test]
fn every_token_comes_from_the_base_theme_at_first() {
    let design = design();
    for (id, _) in SCHEMA.tokens() {
        assert_eq!(design.origin(id), Origin::Base);
    }
    assert_eq!(design.revision(), 0);
}

#[test]
fn snapshots_read_like_their_source_and_capture_again_in_place() {
    let mut design = design();
    let snapshot = design.snapshot();
    assert_eq!(snapshot.len(), SCHEMA.len());
    assert_eq!(
        snapshot.get(component::BUTTON_BACKGROUND_HOVER),
        design.get(component::BUTTON_BACKGROUND_HOVER)
    );
    assert_eq!(
        snapshot.get(primitive::DURATION_FAST),
        Duration::from_millis(100)
    );
    // Captured again after a change, a snapshot shows it; the earlier one keeps its values.
    let red = hex(0xFF_00_00);
    design
        .set_value(Scope::Global, primitive::COLOR_BLUE_700, red)
        .unwrap();
    let mut again = snapshot.clone();
    design.snapshot_into(&mut again);
    assert_eq!(again.get(component::BUTTON_BACKGROUND_HOVER), red);
    assert_ne!(snapshot.get(component::BUTTON_BACKGROUND_HOVER), red);
    // A view captures its own layers: here, the definitions without the override.
    let view = design
        .view(design.active_theme(), Layers::NONE)
        .expect("live theme");
    view.snapshot_into(&mut again);
    assert_eq!(
        again.get(component::BUTTON_BACKGROUND_HOVER),
        hex(0x5D_89_FF)
    );
    // A snapshot of a snapshot is a copy.
    assert_eq!(again.snapshot(), again);
}

//! Value constraints: what each kind of range accepts, how every layer is held to it, and how
//! resolved values stay inside a token's range whatever it aliases.

use koebi_core::Color;
use koebi_design::value::Range;
use koebi_design::{DesignSystem, Error, Scope};

mod primitive {
    use koebi_design::define::*;

    koebi_design::tokens! {
        group SCALE: Primitive, start = 0;

        /// A size that never vanishes.
        SIZE_UNIT = "size.unit": Dimension = px(20.0), positive();
        /// A spacing from zero up, without a ceiling.
        SPACING_WIDE = "spacing.wide": Dimension = px(400.0), at_least(0.0);
        /// A weight by hundreds.
        WEIGHT_STEP = "weight.step": Number = 400.0, steps(100.0, 900.0, 100.0);
        /// A large number the component below may not reach.
        NUMBER_LARGE = "number.large": Number = 50.0, range(0.0, 100.0);
        /// A translucent color.
        COLOR_VEIL = "color.veil": Color = rgba(0, 0, 0, 0.5);
        /// An opaque color.
        COLOR_PAPER = "color.paper": Color = hex("#FAFAFA");
    }
}

mod semantic {
    use super::primitive;

    koebi_design::tokens! {
        group ROLES: Semantic, start = primitive::SCALE::END;

        /// The paper color.
        COLOR_SURFACE = "color.surface": Color -> primitive::COLOR_PAPER;
        /// The veil color.
        COLOR_SHADE = "color.shade": Color -> primitive::COLOR_VEIL;
        /// The large number.
        NUMBER_MANY = "number.many": Number -> primitive::NUMBER_LARGE;
    }
}

mod component {
    use super::semantic;

    koebi_design::tokens! {
        group PANEL: Component, start = semantic::ROLES::END;

        /// A background that must stay opaque.
        PANEL_BACKGROUND = "panel.background": Color -> semantic::COLOR_SURFACE, opaque();
        /// An overlay whose alpha stays between a tenth and a half.
        PANEL_OVERLAY = "panel.overlay": Color -> semantic::COLOR_SHADE, alpha(0.1, 0.5);
        /// A factor that accepts less than what it aliases.
        PANEL_FACTOR = "panel.factor": Number -> semantic::NUMBER_MANY, range(0.0, 10.0);
    }
}

static SCHEMA: koebi_design::schema::Schema = koebi_design::schema::Schema::new(
    "constraints",
    koebi_design::schema::ThemeInfo::new("base", "Base", "Base theme.", &[]),
    &[
        primitive::SCALE::GROUP,
        semantic::ROLES::GROUP,
        component::PANEL::GROUP,
    ],
    &[],
);
const _: () = SCHEMA.assert_valid();

fn design() -> DesignSystem {
    DesignSystem::new(&SCHEMA).expect("the test schema is valid")
}

/// The range in an out-of-range error, if that is the error.
fn refused(result: koebi_design::Result<()>) -> Option<Range> {
    match result.err() {
        Some(Error::OutOfRange { range, .. }) => Some(range),
        _ => None,
    }
}

#[test]
fn positive_and_open_ended_ranges_refuse_what_lies_outside() {
    let mut design = design();
    let range = refused(design.set_value(Scope::Global, primitive::SIZE_UNIT, 0.0)).unwrap();
    assert_eq!(range.to_string(), "above 0");
    design
        .set_value(Scope::Global, primitive::SIZE_UNIT, 0.5)
        .unwrap();
    // No ceiling, but a floor.
    design
        .set_value(Scope::Global, primitive::SPACING_WIDE, 1.0e6)
        .unwrap();
    let range = refused(design.set_value(Scope::Global, primitive::SPACING_WIDE, -1.0)).unwrap();
    assert_eq!(range.to_string(), "at least 0");
}

#[test]
fn stepped_ranges_accept_their_steps_only() {
    let mut design = design();
    design
        .set_value(Scope::Global, primitive::WEIGHT_STEP, 700.0)
        .unwrap();
    let range = refused(design.set_value(Scope::Global, primitive::WEIGHT_STEP, 650.0)).unwrap();
    assert_eq!(range.to_string(), "100 to 900 in steps of 100");
    refused(design.set_value(Scope::Global, primitive::WEIGHT_STEP, 1000.0)).unwrap();
}

#[test]
fn color_ranges_bound_the_alpha() {
    let mut design = design();
    let translucent = Color::new(1.0, 1.0, 1.0, 0.8);
    let range =
        refused(design.set_value(Scope::Global, component::PANEL_BACKGROUND, translucent)).unwrap();
    assert_eq!((range.min(), range.max()), (1.0, 1.0));
    design
        .set_value(Scope::Global, component::PANEL_BACKGROUND, Color::BLACK)
        .unwrap();
    refused(design.set_value(Scope::Global, component::PANEL_OVERLAY, translucent)).unwrap();
    design
        .set_value(
            Scope::Global,
            component::PANEL_OVERLAY,
            Color::new(0.0, 0.0, 0.0, 0.3),
        )
        .unwrap();
}

#[test]
fn resolved_values_stay_inside_the_range_whatever_they_alias() {
    let mut design = design();
    // The factor aliases a number of 50 but accepts 0 to 10: it reads 10.
    assert_eq!(design.get(component::PANEL_FACTOR), 10.0);
    design
        .set_value(Scope::Global, primitive::NUMBER_LARGE, 4.0)
        .unwrap();
    assert_eq!(design.get(component::PANEL_FACTOR), 4.0);
    // An opaque background aliasing a translucent color reads it opaque.
    design
        .set_alias(
            Scope::Global,
            component::PANEL_BACKGROUND,
            semantic::COLOR_SHADE,
        )
        .unwrap();
    assert_eq!(design.get(component::PANEL_BACKGROUND).a, 1.0);
    // And the overlay's alpha stays within its bounds when its target turns opaque.
    design
        .set_value(Scope::Global, primitive::COLOR_VEIL, Color::BLACK)
        .unwrap();
    assert_eq!(design.get(component::PANEL_OVERLAY).a, 0.5);
}

//! Tests of expressions: numbers, operators and precedence, functions, constants, units and
//! conversions, and errors.

use super::*;

fn plain(text: &str) -> f64 {
    evaluate(text, &Context::default()).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn in_unit(text: &str, unit: &Unit) -> f64 {
    evaluate(text, &Context::in_unit(unit)).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn error(text: &str, context: &Context<'_>) -> ExprError {
    evaluate(text, context).expect_err(text)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

#[test]
fn numbers_read_in_every_form() {
    assert_eq!(plain("12"), 12.0);
    assert_eq!(plain("1.5"), 1.5);
    assert_eq!(plain(".5"), 0.5);
    assert_eq!(plain("1e3"), 1000.0);
    assert_eq!(plain("2.5E-1"), 0.25);
    assert_eq!(plain("  7  "), 7.0);
    // A decimal comma, outside function arguments.
    assert_eq!(plain("1,5"), 1.5);
    assert_eq!(plain("(2,25)*2"), 4.5);
    assert_eq!(plain("max(1,5)"), 5.0);
}

#[test]
fn operators_follow_precedence_and_associativity() {
    assert_eq!(plain("2+3*4"), 14.0);
    assert_eq!(plain("(2+3)*4"), 20.0);
    assert_eq!(plain("10-4-3"), 3.0);
    assert_eq!(plain("100/10/5"), 2.0);
    // Powers go through `powf`, whose precision Rust leaves open (Miri perturbs it).
    assert!(close(plain("2^3^2"), 512.0));
    assert!(close(plain("-2^2"), -4.0));
    assert!(close(plain("2^-1"), 0.5));
    assert_eq!(plain("--3"), 3.0);
    assert_eq!(plain("+3"), 3.0);
    assert_eq!(plain("2*(3+4)"), 14.0);
    // Implicit multiplication.
    assert_eq!(plain("2(3+4)"), 14.0);
    assert!(close(plain("2pi"), core::f64::consts::TAU));
    assert_eq!(plain("3 sqrt(4)"), 6.0);
    // Typographic operators.
    assert_eq!(plain("6 \u{d7} 7 \u{2212} 2 \u{f7} 2"), 41.0);
}

#[test]
fn functions_and_constants_evaluate() {
    assert!(close(plain("pi"), core::f64::consts::PI));
    assert!(close(plain("tau/2"), core::f64::consts::PI));
    assert!(close(plain("e"), core::f64::consts::E));
    assert_eq!(plain("sqrt(16)"), 4.0);
    assert_eq!(plain("abs(-3)"), 3.0);
    assert_eq!(plain("floor(2.7) + ceil(2.1) + round(2.5)"), 8.0);
    assert_eq!(plain("min(4, 2, 8)"), 2.0);
    assert_eq!(plain("max(4, 2, 8)"), 8.0);
    assert_eq!(plain("mod(-1, 3)"), 2.0);
    assert!(close(plain("exp(ln(5))"), 5.0));
    assert!(close(plain("log(1000)"), 3.0));
    assert!(close(plain("log2(8)"), 3.0));
    assert!(close(plain("sin(pi/2)"), 1.0));
    assert!(close(plain("cos(0)"), 1.0));
    assert!(close(plain("tan(pi/4)"), 1.0));
}

#[test]
fn units_convert_to_the_field_unit() {
    let centimeter = unit("cm").expect("a unit");
    assert!(close(in_unit("10mm", centimeter), 1.0));
    assert!(close(in_unit("1m 20cm", centimeter), 120.0));
    assert!(close(in_unit("(1+2)cm", &MILLIMETER), 30.0));
    assert!(close(in_unit("1in", &MILLIMETER), 25.4));
    // Pixels at 96 per inch: an inch is 96 pixels, a point 4/3 of a pixel.
    assert!(close(in_unit("1in", &PIXEL), 96.0));
    assert!(close(in_unit("3pt", &PIXEL), 4.0));
    let print = Context {
        unit: Some(&PIXEL),
        pixels_per_inch: 300.0,
    };
    assert!(close(evaluate("1in", &print).unwrap(), 300.0));
    // Angles and times.
    assert!(close(
        in_unit("90deg", &RADIAN),
        core::f64::consts::FRAC_PI_2
    ));
    assert!(close(
        in_unit("90\u{b0}", &RADIAN),
        core::f64::consts::FRAC_PI_2
    ));
    assert!(close(in_unit("pi rad", &DEGREE), 180.0));
    assert!(close(in_unit("0.25turn", &DEGREE), 90.0));
    assert!(close(in_unit("1min 30s", &SECOND), 90.0));
    assert!(close(in_unit("250ms", &SECOND), 0.25));
}

#[test]
fn numbers_without_a_unit_count_in_the_field_unit() {
    // A plain result is already in the field's unit.
    assert_eq!(in_unit("12", &MILLIMETER), 12.0);
    // Added to a length, a plain number is a length in the field's unit.
    assert!(close(in_unit("1cm + 5", &MILLIMETER), 15.0));
    assert!(close(in_unit("5 + 1cm", &MILLIMETER), 15.0));
    // Factors stay plain.
    assert!(close(in_unit("2 * 1cm", &MILLIMETER), 20.0));
    assert!(close(in_unit("1cm / 2", &MILLIMETER), 5.0));
    // A ratio of lengths is plain: in the field's unit as such.
    assert!(close(in_unit("10mm / 1cm", &MILLIMETER), 1.0));
    // Rounding happens in the field's unit.
    assert!(close(in_unit("round(1.26cm)", &MILLIMETER), 13.0));
    assert!(close(in_unit("min(1cm, 15)", &MILLIMETER), 10.0));
    // Trigonometry reads angles in their unit, plain numbers in radians.
    assert!(close(plain("sin(30deg)"), 0.5));
    assert!(close(in_unit("asin(1)", &DEGREE), 90.0));
    assert!(close(in_unit("atan2(1cm, 1cm)", &DEGREE), 45.0));
    // Squares and roots of lengths: a power after a unit raises the unit alone.
    assert!(close(in_unit("sqrt(4cm^2)", &MILLIMETER), 20.0));
    assert!(close(in_unit("sqrt((4cm)^2)", &MILLIMETER), 40.0));
    assert!(close(in_unit("1m^2 / 1cm", &MILLIMETER), 100_000.0));
}

#[test]
fn errors_say_what_is_wrong_and_where() {
    let plain_field = Context::default();
    let lengths = Context::in_unit(&MILLIMETER);
    let kind = |text: &str, context: &Context<'_>| error(text, context).kind;
    assert_eq!(kind("", &plain_field), ErrorKind::Empty);
    assert_eq!(kind("   ", &plain_field), ErrorKind::Empty);
    assert_eq!(
        error("2 $ 3", &plain_field),
        ExprError {
            kind: ErrorKind::UnexpectedCharacter,
            start: 2,
            end: 3,
        }
    );
    assert_eq!(kind("2 +", &plain_field), ErrorKind::UnexpectedEnd);
    assert_eq!(kind("2 3", &plain_field), ErrorKind::Unexpected);
    assert_eq!(kind("(2+3))", &plain_field), ErrorKind::Unexpected);
    assert_eq!(
        error("(2+3", &plain_field),
        ExprError {
            kind: ErrorKind::UnclosedParenthesis,
            start: 0,
            end: 1,
        }
    );
    assert_eq!(kind("sqrt(4", &plain_field), ErrorKind::UnclosedParenthesis);
    assert_eq!(
        error("2 * foo", &plain_field),
        ExprError {
            kind: ErrorKind::UnknownName,
            start: 4,
            end: 7,
        }
    );
    assert_eq!(
        kind("sqrt(1, 2)", &plain_field),
        ErrorKind::WrongArgumentCount
    );
    assert_eq!(kind("mod(1)", &plain_field), ErrorKind::WrongArgumentCount);
    assert_eq!(kind("min()", &plain_field), ErrorKind::WrongArgumentCount);
    assert_eq!(kind("1/0", &plain_field), ErrorKind::NotFinite);
    assert_eq!(kind("sqrt(-1)", &plain_field), ErrorKind::NotFinite);
    assert_eq!(kind("mod(3, 0)", &plain_field), ErrorKind::NotFinite);
    // Units that do not fit.
    assert_eq!(kind("1cm", &plain_field), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("90deg", &lengths), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("1cm + 1s", &lengths), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("1m 30s", &lengths), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("1cm * 1cm", &lengths), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("2^1cm", &lengths), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("sqrt(1cm)", &lengths), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("exp(1cm)", &lengths), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("(2mm)cm", &lengths), ErrorKind::IncompatibleUnits);
    assert_eq!(kind("2cm^0.5", &lengths), ErrorKind::IncompatibleUnits);
    // A unit needs a value, and side by side quantities need units.
    assert_eq!(kind("mm", &lengths), ErrorKind::Unexpected);
    assert_eq!(kind("2 mm cm", &lengths), ErrorKind::Unexpected);
    assert_eq!(kind("1m 20", &lengths), ErrorKind::Unexpected);
    assert_eq!(
        error("1/0", &plain_field).to_string(),
        "no finite result at 1..2"
    );
}

#[test]
fn units_are_found_by_any_of_their_names() {
    assert_eq!(unit("millimeters"), Some(&MILLIMETER));
    assert_eq!(unit("\u{b0}"), Some(&DEGREE));
    assert_eq!(unit("deg").map(|unit| unit.symbol), Some("\u{b0}"));
    assert_eq!(unit("parsec"), None);
    // Every name belongs to one unit only.
    for (index, unit) in UNITS.iter().enumerate() {
        for name in unit.names {
            let owners = UNITS
                .iter()
                .filter(|other| other.names.contains(name))
                .count();
            assert_eq!(owners, 1, "{name} of unit {index}");
        }
    }
}

//! Persistence: saving and restoring every kind of state, files, backups and debouncing.

#[allow(dead_code, reason = "each test crate uses part of the shared schema")]
mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use common::{component, hex, primitive, semantic};
use koebi_design::persist::{Store, encode, restore};
use koebi_design::value::{CubicBezier, Curve, CurveKey, Definition, Value};
use koebi_design::{ColorVision, Deficiency, DesignSystem, Error, Layers, NewTheme, Scope};

static SCHEMA: koebi_design::schema::Schema = common::SCHEMA;

fn design() -> DesignSystem {
    DesignSystem::new(&SCHEMA).expect("the test schema is valid")
}

/// A design system with something in every saved layer.
fn customized() -> DesignSystem {
    let mut design = design();
    let red = design
        .create_theme(
            NewTheme::new("user.red", "Red")
                .with(
                    semantic::ACCENT_COLOR_700,
                    Definition::Alias(primitive::COLOR_RED_700.id()),
                )
                .with(
                    semantic::RADIUS_CONTROL,
                    Definition::Value(Value::Dimension(6.0)),
                ),
        )
        .expect("valid customization");
    let light = design.find_theme("light").expect("valid customization");
    design
        .set_value(
            Scope::Theme(light),
            semantic::BACKGROUND_BASE,
            hex(0xFF_FF_FF),
        )
        .expect("valid customization");
    design
        .set_alias(
            Scope::Theme(red),
            component::BUTTON_BACKGROUND_HOVER,
            component::LOGO,
        )
        .expect("valid customization");
    design
        .set_value(Scope::Global, primitive::FONT_FAMILY_SANS, "Inter".into())
        .expect("valid customization");
    design
        .set_value(
            Scope::Global,
            primitive::DURATION_FAST,
            Duration::from_millis(120),
        )
        .expect("valid customization");
    design
        .set_alias(Scope::Global, semantic::CHAIN_1, primitive::COLOR_GREEN_700)
        .expect("valid customization");
    design
        .set_value(
            Scope::Global,
            primitive::CURVE_SETTLE,
            Curve::new(&[
                CurveKey::new(0.0, 0.0, CubicBezier::new(0.1, 0.0, 0.3, 1.0)),
                CurveKey::new(0.5, 0.9, CubicBezier::LINEAR),
                CurveKey::new(0.8, 1.05, CubicBezier::new(0.5, 0.0, 0.5, 1.0)),
                CurveKey::new(1.0, 1.0, CubicBezier::LINEAR),
            ]),
        )
        .expect("valid customization");
    design
        .set_token_layers(component::LOGO, Layers::ALL - Layers::COLOR_VISION)
        .expect("valid customization");
    design
        .set_color_vision(ColorVision::Correct {
            deficiency: Deficiency::Protan,
            strength: 0.8,
        })
        .expect("valid customization");
    design.set_active_theme(red).expect("valid customization");
    design
}

fn all_values(design: &DesignSystem) -> Vec<Value> {
    SCHEMA
        .tokens()
        .map(|(id, _)| design.value(id).expect("every token resolves").clone())
        .collect()
}

/// A fresh directory under the system's temporary directory, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("koebi-design-{}-{name}", std::process::id()));
        // A leftover from an interrupted run, if any.
        std::fs::remove_dir_all(&path).ok();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best effort: a leftover temporary directory is harmless.
        std::fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn every_saved_layer_round_trips() {
    let original = customized();
    let bytes = encode(&original);
    let mut restored = design();
    let report = restore(&mut restored, &bytes).unwrap();
    assert!(report.is_complete(), "{:?}", report.skipped);

    assert_eq!(
        restored.theme(restored.active_theme()).unwrap().key(),
        "user.red"
    );
    assert_eq!(restored.color_vision(), original.color_vision());
    assert_eq!(
        restored.token_layers(component::LOGO),
        Layers::ALL - Layers::COLOR_VISION
    );
    // Every token resolves to the same value, in the active theme...
    assert_eq!(all_values(&restored), all_values(&original));
    // ...and in the other themes (compared without the color vision layer, which transforms
    // white within rounding error).
    let light = restored.find_theme("light").unwrap();
    assert_eq!(
        restored
            .view(light, Layers::DEFINITIONS)
            .unwrap()
            .get(semantic::BACKGROUND_BASE),
        hex(0xFF_FF_FF)
    );
    // The same state always encodes to the same bytes.
    assert_eq!(encode(&restored), bytes);
}

#[test]
fn aliases_restore_whatever_their_order() {
    let mut original = design();
    // CHAIN_1 aliases CHAIN_2, which only works once CHAIN_2 holds a value: the restore must
    // apply values first, then aliases in an order that keeps every step valid.
    original
        .set_value(Scope::ActiveTheme, semantic::CHAIN_2, hex(0xFF_FF_FF))
        .unwrap();
    original
        .set_alias(Scope::ActiveTheme, semantic::CHAIN_1, semantic::CHAIN_2)
        .unwrap();
    original
        .set_alias(Scope::Global, component::BUTTON_BACKGROUND, component::DEEP)
        .unwrap();
    let mut restored = design();
    let report = restore(&mut restored, &encode(&original)).unwrap();
    assert!(report.is_complete(), "{:?}", report.skipped);
    assert_eq!(all_values(&restored), all_values(&original));
}

#[test]
fn damaged_data_is_rejected_without_changes() {
    let bytes = encode(&customized());
    let mut target = design();

    let mut flipped = bytes.clone();
    let last = flipped.len() - 1;
    flipped[last] ^= 0x40;
    assert!(matches!(
        restore(&mut target, &flipped),
        Err(Error::Corrupt(_))
    ));
    assert!(matches!(
        restore(&mut target, &bytes[..bytes.len() - 3]),
        Err(Error::Corrupt(_))
    ));
    assert!(matches!(
        restore(&mut target, b"not a design file at all"),
        Err(Error::Corrupt(_))
    ));
    let mut newer = bytes.clone();
    newer[8] = 2;
    assert!(matches!(
        restore(&mut target, &newer),
        Err(Error::UnsupportedVersion(2))
    ));
    assert_eq!(target.revision(), 0, "nothing was applied");
}

#[test]
fn the_store_saves_only_changes_and_restores_them() {
    let dir = TempDir::new("round-trip");
    let mut design = customized();
    let mut store = Store::new(&dir.0);
    assert!(store.save(&design).unwrap());
    assert!(!store.save(&design).unwrap(), "nothing changed");
    assert!(store.path().exists());

    let mut reopened = self::design();
    let report = Store::new(&dir.0).load(&mut reopened).unwrap().unwrap();
    assert!(report.is_complete());
    assert_eq!(all_values(&reopened), all_values(&design));

    design
        .set_value(Scope::Global, primitive::RADIUS_100, 3.0)
        .unwrap();
    assert!(store.is_dirty(&design));
    assert!(store.save(&design).unwrap());
}

#[test]
fn the_store_loads_nothing_before_the_first_save() {
    let dir = TempDir::new("empty");
    let mut design = design();
    let mut store = Store::new(&dir.0);
    assert_eq!(store.load(&mut design).unwrap(), None);
    assert!(!store.is_dirty(&design));
}

#[test]
fn a_damaged_file_falls_back_to_the_backup() {
    let dir = TempDir::new("backup");
    let mut design = customized();
    let mut store = Store::new(&dir.0);
    store.save(&design).unwrap();
    design
        .set_value(Scope::Global, primitive::RADIUS_100, 3.0)
        .unwrap();
    store.save(&design).unwrap();
    // The newest file is damaged: the previous state comes back from the backup.
    std::fs::write(store.path(), b"KOEBIDS\0garbage").unwrap();
    let mut reopened = self::design();
    let report = Store::new(&dir.0).load(&mut reopened).unwrap().unwrap();
    assert!(report.skipped[0].contains("restored the backup"));
    assert_eq!(reopened.get(primitive::RADIUS_100), 2.0);
    assert_eq!(
        reopened.get(primitive::DURATION_FAST),
        Duration::from_millis(120)
    );

    // A crash between the two renames of a save leaves only the backup.
    std::fs::remove_file(store.path()).unwrap();
    let mut recovered = self::design();
    assert!(Store::new(&dir.0).load(&mut recovered).unwrap().is_some());
    assert_eq!(
        recovered.get(primitive::DURATION_FAST),
        Duration::from_millis(120)
    );
}

#[test]
fn saves_wait_for_a_pause() {
    let dir = TempDir::new("debounce");
    let mut design = design();
    let mut store = Store::new(&dir.0).with_delay(Duration::from_millis(500));
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);

    // A slider drags for a second: one change per frame.
    for frame in 0..60_u64 {
        let radius = 2.0 + frame as f32 * 0.1;
        design
            .set_value(Scope::Global, primitive::RADIUS_100, radius)
            .unwrap();
        assert!(!store.save_if_due(&design, at(frame * 16)).unwrap() || frame * 16 >= 500);
    }
    // Still saved at most once per pause, and the final value reaches the disk.
    store.save_if_due(&design, at(5000)).unwrap();
    assert!(!store.is_dirty(&design));
    let mut reopened = self::design();
    Store::new(&dir.0).load(&mut reopened).unwrap();
    assert_eq!(
        reopened.get(primitive::RADIUS_100),
        design.get(primitive::RADIUS_100)
    );
}

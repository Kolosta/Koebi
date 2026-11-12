//! Themes generated from chosen colors (ADR 0027): each rule of `koebi_tokens::generate`, on
//! the built-in themes. `tests/contrast.rs` checks every component's contrast over a grid of
//! backgrounds, accents and modes.

// Token constants of the last groups sit at the end of a chain of group starts: evaluating
// them goes deeper than the default limit on some hosts.
#![recursion_limit = "256"]

use koebi_core::{Color, Oklch};
use koebi_design::value::Definition;
use koebi_design::{DesignSystem, Layers, Scope, ThemeId};
use koebi_tokens::generate::{self, BackgroundFit, LIGHT, Mode, ROLE_FILLS};
use koebi_tokens::palettes::{self, Editable, Family};
use koebi_tokens::{KOEBI, primitive, semantic};

fn design() -> DesignSystem {
    DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
}

fn theme(design: &mut DesignSystem, key: &str) -> ThemeId {
    let theme = design.find_theme(key).expect("a built-in theme");
    design
        .set_active_theme(theme)
        .expect("a built-in theme can be active");
    theme
}

fn palette(key: &str) -> Editable {
    palettes::editable()
        .find(|palette| palette.key == key)
        .expect("an editable palette")
}

fn neutral() -> Editable {
    palettes::editable()
        .find(|palette| palette.family == Family::Neutral)
        .expect("the neutral palette")
}

/// The step a token aliases in `theme`, among `steps`.
fn step_of(
    design: &DesignSystem,
    theme: ThemeId,
    token: koebi_design::TokenId,
    steps: &[koebi_design::TokenId; 16],
) -> Option<usize> {
    let view = design.view(theme, Layers::DEFINITIONS)?;
    let target = view.definition(token)?.alias_target()?;
    steps.iter().position(|&step| step == target)
}

#[test]
fn a_theme_with_nothing_chosen_holds_no_override() {
    let mut design = design();
    let themes: Vec<ThemeId> = design.themes().map(|(id, _)| id).collect();
    for theme in themes {
        let plan = generate::plan(&design, theme).unwrap();
        assert!(plan.is_empty(), "{theme:?}: {} outputs", plan.len());
        // The background modes on a gray background change nothing either: no hue to follow.
        for mode in [Mode::Background, Mode::BackgroundAccent] {
            generate::set_mode(&mut design, theme, mode).unwrap();
            let outputs = generate::plan(&design, theme).unwrap().len();
            assert_eq!(outputs, 0, "{theme:?} {mode:?}");
        }
        generate::set_mode(&mut design, theme, Mode::Palettes).unwrap();
        assert_eq!(design.overrides(Scope::Theme(theme)).unwrap().count(), 0);
    }
}

#[test]
fn the_light_structure_is_snow_s_and_arctic_s() {
    let design = design();
    for key in ["snow", "arctic"] {
        let theme = design.find_theme(key).unwrap();
        let view = design.view(theme, Layers::THEME).unwrap();
        for (token, target) in LIGHT {
            let definition = view.definition(token).and_then(Definition::alias_target);
            assert_eq!(definition, Some(target), "{key}: {:?}", design.info(token));
        }
        // Their states go one and two steps away from the background, the dark base's toward it.
        for fills in &ROLE_FILLS {
            let default = step_of(&design, theme, fills.default, &fills.steps).unwrap();
            let hover = step_of(&design, theme, fills.hover[0], &fills.steps).unwrap();
            let pressed = step_of(&design, theme, fills.pressed[0], &fills.steps).unwrap();
            assert_eq!(
                (hover, pressed),
                (default + 1, default + 2),
                "{key} {}",
                fills.key
            );
        }
    }
    let dark = design.find_theme("dark").unwrap();
    for fills in &ROLE_FILLS {
        let default = step_of(&design, dark, fills.default, &fills.steps).unwrap();
        let hover = step_of(&design, dark, fills.hover[0], &fills.steps).unwrap();
        let pressed = step_of(&design, dark, fills.pressed[0], &fills.steps).unwrap();
        assert_eq!(
            (hover, pressed),
            (default - 1, default - 2),
            "dark {}",
            fills.key
        );
    }
}

#[test]
fn a_chosen_background_is_exact_moved_out_of_the_mid_tones_or_the_deepest_surface() {
    let mut design = design();
    let dark = theme(&mut design, "dark");
    let neutral = neutral();
    let base = || semantic::BACKGROUND_BASE_DEFAULT;
    // Exact: a dark blue-gray.
    let chosen = Color::from_hex(0x1F_24_30);
    palettes::seed(&mut design, dark, &neutral, chosen).unwrap();
    assert_eq!(
        generate::report(&design, dark).background,
        Some(BackgroundFit::Exact)
    );
    assert_eq!(design.get(base()).to_rgba8(), chosen.to_rgba8());
    // A mid-gray holds no text at 12:1: it moves to the nearest readable lightness.
    let gray = Color::from_hex(0x80_80_80);
    palettes::seed(&mut design, dark, &neutral, gray).unwrap();
    let Some(BackgroundFit::Moved(moved)) = generate::report(&design, dark).background else {
        panic!("a mid-gray moves");
    };
    assert_eq!(design.get(base()).to_rgba8(), moved.to_rgba8());
    let room = Color::WHITE
        .contrast_ratio(moved)
        .max(Color::BLACK.contrast_ratio(moved));
    assert!(room >= generate::rules::TEXT_ROOM - 0.05, "{room}");
    // Pure black leaves no room for a deeper surface: it is the deepest one, the background
    // lies at the theme's contrast from it, which is the dark theme's own background.
    palettes::seed(&mut design, dark, &neutral, Color::BLACK).unwrap();
    assert_eq!(
        generate::report(&design, dark).background,
        Some(BackgroundFit::Deepest)
    );
    assert_eq!(design.get(primitive::COLOR_GRAY_25), Color::BLACK);
    let ratio = design.get(base()).contrast_ratio(Color::BLACK);
    assert!((ratio - 1.13).abs() < 0.01, "{ratio}");
}

#[test]
fn a_light_background_gives_a_dark_theme_the_light_structure() {
    let mut design = design();
    let dark = theme(&mut design, "dark");
    let raised = semantic::BACKGROUND_RAISED_DEFAULT;
    let before = design.get(raised);
    palettes::seed(&mut design, dark, &neutral(), Color::from_hex(0xF2_F0_EC)).unwrap();
    // Raised surfaces turn to the lighter step, as in Snow and Arctic.
    assert_eq!(design.get(raised), design.get(primitive::COLOR_GRAY_25));
    // States go away from the light background: one and two steps up.
    let accent = &ROLE_FILLS[0];
    let default = step_of(&design, dark, accent.default, &accent.steps).unwrap();
    let hover = step_of(&design, dark, accent.hover[0], &accent.steps).unwrap();
    let pressed = step_of(&design, dark, accent.pressed[0], &accent.steps).unwrap();
    assert_eq!((hover, pressed), (default + 1, default + 2));
    // A reset gives the dark structure back, with no override left.
    palettes::reset(&mut design, dark, &neutral()).unwrap();
    assert_eq!(design.get(raised), before);
    assert_eq!(design.overrides(Scope::Theme(dark)).unwrap().count(), 0);
}

#[test]
fn a_chosen_accent_is_its_fill_or_moves_to_the_fill_s_contrast() {
    let mut design = design();
    let dark = theme(&mut design, "dark");
    let accent = palette("accent");
    let fill = semantic::BACKGROUND_ACCENT_DEFAULT;
    // A light orange is the fill itself, its content dark.
    let orange = Color::from_hex(0xFF_A0_40);
    palettes::seed(&mut design, dark, &accent, orange).unwrap();
    assert_eq!(design.get(fill).to_rgba8(), orange.to_rgba8());
    assert_eq!(
        design.get(semantic::TEXT_COLOR_ON_ACCENT_DEFAULT),
        design.get(semantic::STATIC_TEXT_COLOR_BLACK)
    );
    assert_eq!(generate::report(&design, dark).moved, []);
    // A navy too close to the dark background moves along its scale to 3.96:1, the dark
    // theme's own fill contrast, and the settings are told.
    let navy = Color::from_hex(0x1A_22_40);
    palettes::seed(&mut design, dark, &accent, navy).unwrap();
    let report = generate::report(&design, dark);
    assert_eq!(report.moved.len(), 1);
    assert_eq!(report.moved[0].0, "accent");
    let background = design.get(semantic::BACKGROUND_BASE_DEFAULT);
    let ratio = design.get(fill).contrast_ratio(background);
    assert!((ratio - 3.96).abs() < 0.05, "{ratio}");
    let hue = design.get(fill).to_oklch().h;
    assert!(
        (hue - navy.to_oklch().h).abs() < 8.0,
        "the navy's hue: {hue}"
    );
}

#[test]
fn the_background_modes_make_the_accent_and_the_roles_follow_the_background() {
    let mut design = design();
    let dark = theme(&mut design, "dark");
    let teal = Oklch::new(0.25, 0.05, 200.0).to_color();
    generate::set_mode(&mut design, dark, Mode::Background).unwrap();
    palettes::seed(&mut design, dark, &neutral(), teal).unwrap();
    // The accent takes the background's hue.
    let accent = design.get(semantic::BACKGROUND_ACCENT_DEFAULT).to_oklch();
    assert!((accent.h - 200.0).abs() < 10.0, "{}", accent.h);
    // The other roles turn toward it, by half the difference, 15 degrees at most.
    let own = design
        .view(dark, Layers::THEME)
        .and_then(|view| view.value(semantic::NEGATIVE_COLOR_900.id()))
        .and_then(|value| match value {
            koebi_design::value::Value::Color(color) => Some(color.to_oklch().h),
            _ => None,
        })
        .unwrap();
    let negative = design
        .get(semantic::BACKGROUND_NEGATIVE_DEFAULT)
        .to_oklch()
        .h;
    let turned = (negative - own + 540.0).rem_euclid(360.0) - 180.0;
    assert!(turned.abs() > 1.0 && turned.abs() <= 15.5, "{turned}");
    // Back to every palette from its own color: the derived palettes go, the background stays.
    generate::set_mode(&mut design, dark, Mode::Palettes).unwrap();
    let scope = Scope::Theme(dark);
    for fills in &ROLE_FILLS[..5] {
        for &step in &fills.steps {
            assert!(design.override_of(scope, step).is_none(), "{}", fills.key);
        }
    }
    assert_eq!(
        design.get(semantic::BACKGROUND_BASE_DEFAULT).to_rgba8(),
        teal.to_rgba8()
    );
}

#[test]
fn the_background_and_accent_mode_keeps_the_chosen_accent() {
    let mut design = design();
    let dark = theme(&mut design, "dark");
    let violet = Color::from_hex(0x9B_6B_FF);
    generate::set_mode(&mut design, dark, Mode::BackgroundAccent).unwrap();
    palettes::seed(&mut design, dark, &palette("accent"), violet).unwrap();
    palettes::seed(
        &mut design,
        dark,
        &neutral(),
        Oklch::new(0.22, 0.04, 140.0).to_color(),
    )
    .unwrap();
    assert_eq!(
        design.get(semantic::BACKGROUND_ACCENT_DEFAULT).to_rgba8(),
        violet.to_rgba8()
    );
    // In the background mode, the chosen accent waits: the accent follows the background.
    generate::set_mode(&mut design, dark, Mode::Background).unwrap();
    let hue = design.get(semantic::BACKGROUND_ACCENT_DEFAULT).to_oklch().h;
    assert!((hue - 140.0).abs() < 10.0, "{hue}");
    generate::set_mode(&mut design, dark, Mode::BackgroundAccent).unwrap();
    assert_eq!(
        design.get(semantic::BACKGROUND_ACCENT_DEFAULT).to_rgba8(),
        violet.to_rgba8()
    );
}

#[test]
fn a_hand_edit_of_a_token_the_generator_does_not_write_stays() {
    let mut design = design();
    let dark = theme(&mut design, "dark");
    let scope = Scope::Theme(dark);
    let raised = semantic::BACKGROUND_RAISED_DEFAULT;
    design
        .set_alias(scope, raised, primitive::COLOR_GRAY_200)
        .unwrap();
    palettes::seed(
        &mut design,
        dark,
        &palette("accent"),
        Color::from_hex(0x40_A0_FF),
    )
    .unwrap();
    palettes::reset(&mut design, dark, &palette("accent")).unwrap();
    assert_eq!(design.get(raised), design.get(primitive::COLOR_GRAY_200));
}

#[test]
fn the_mode_is_a_token_saved_with_the_theme() {
    let mut design = design();
    let dark = theme(&mut design, "dark");
    assert_eq!(Mode::of(&design, dark), Mode::Palettes);
    generate::set_mode(&mut design, dark, Mode::BackgroundAccent).unwrap();
    assert_eq!(Mode::of(&design, dark), Mode::BackgroundAccent);
    let exported = koebi_design::dtcg::export_theme(&design, dark).unwrap();
    assert!(exported.contains("\"mode\""), "{exported}");
}

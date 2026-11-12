//! The rules that make a theme's colors from chosen ones (ADR 0027), as functions of colors:
//! no design system here, so that each rule is tested alone. `docs/design-system/color.md`
//! (*Generated themes*) states them with their constants; each function names its rule.

use koebi_core::palette::ColorScale;
use koebi_core::{Color, Oklch};

use crate::palettes::{
    LAST_HUE_FILL, LAST_NEUTRAL_FILL, NEUTRAL_CONTRASTS, Scheme, ThemePalettes, WHITE_CONTENT,
};

/// Rule 2: the contrast the strongest text needs room for. Text lies on surfaces lighter than
/// the background too: the weakest pair is hint text (step 700) on a field (step 300, 2.03:1),
/// which keeps 4.5:1 (WCAG 1.4.3) once the text steps are compressed (rule 4) from 12:1 on.
pub const TEXT_ROOM: f32 = 12.0;

/// Rule 3: the least contrast the deepest surface keeps with the background, the smallest
/// between neighbouring surfaces of the neutral palette (steps 75 and 100: 1.18 / 1.115).
pub const SURFACE_APART: f32 = 1.058;

/// Rules 4 and 5: the contrast a fill keeps with what it lies on, WCAG 1.4.11 for what
/// identifies a control.
pub const FILL_APART: f32 = 3.0;

/// Rules 4 and 5: the least contrast a fill darkened for white content keeps with the
/// background: [`FILL_APART`] against the raised surfaces it lies on (step 150, 1.278:1).
pub fn fill_floor() -> f32 {
    FILL_APART * NEUTRAL_CONTRASTS[4]
}

/// Rule 9: the contrast content needs on its fill, WCAG 1.4.3.
pub const CONTENT: f32 = 4.5;

/// Rule 8: how far a pressed fill keeps from its default before the states turn the other way,
/// below the built-in themes' smallest (1.52:1, the dark accent).
pub const PRESSED_APART: f32 = 1.4;

/// Rule 7: the share of the hue difference a role turns toward the background's, and its limit
/// in degrees.
pub const HARMONY_SHARE: f32 = 0.5;
/// Rule 7: the most a role's hue turns toward the background's.
pub const HARMONY_LIMIT: f32 = 15.0;

/// Rules 7 and 9: the OKLCH chroma below which a color is a gray, with no hue to follow.
pub const HUED: f32 = 0.02;

/// The neutral steps holding text: from 600 (index 11) to 900 (index 14), above step 500.
const TEXT_STEPS: core::ops::RangeInclusive<usize> = 11..=14;

/// What the chosen background became.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackgroundFit {
    /// The window's background, as chosen.
    Exact,
    /// Moved out of the mid-tones, to this color, so that text reads (rule 2).
    Moved(Color),
    /// Too close to black or white for a deeper surface: it is the deepest surface, and the
    /// window's background lies at the theme's contrast from it (rule 3).
    Deepest,
}

/// Rule 2: `background`, or the color of the same hue and chroma nearest in lightness where
/// text can reach `room` (the theme's [`ThemePalettes::text_room`], [`TEXT_ROOM`] but for high
/// contrast): for 12:1, no lighter than luminance 0.0375 for light text, no darker than 0.55
/// for dark text.
pub fn readable(background: Color, room: f32) -> Color {
    let background = background.with_alpha(1.0);
    let luminance = background.relative_luminance();
    let dark = 1.05 / room - 0.05;
    let light = 0.05 * room - 0.05;
    if luminance <= dark || luminance >= light {
        return background;
    }
    let lch = background.to_oklch();
    let darker = with_luminance(lch, dark);
    let lighter = with_luminance(lch, light);
    if (lch.l - darker.to_oklch().l).abs() <= (lighter.to_oklch().l - lch.l).abs() {
        darker
    } else {
        lighter
    }
}

/// The color of `lch`'s hue and chroma whose luminance is `target`, by bisection on lightness.
fn with_luminance(lch: Oklch, target: f32) -> Color {
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..40 {
        let middle = low.midpoint(high);
        if Oklch::new(middle, lch.c, lch.h)
            .to_color()
            .relative_luminance()
            < target
        {
            low = middle;
        } else {
            high = middle;
        }
    }
    // The side that meets the bound: at or below it for the dark side, at or above it for the
    // light one.
    let (low, high) = (
        Oklch::new(low, lch.c, lch.h).to_color(),
        Oklch::new(high, lch.c, lch.h).to_color(),
    );
    if target < 0.18 { low } else { high }
}

/// The color of `scale` between `from` and the end of the scale on `from`'s near side (black
/// for a dark scheme, white for a light one) at contrast `ratio` with `from`, by bisection.
fn toward(scale: &ColorScale, from: Color, ratio: f32, scheme: Scheme) -> Color {
    let start = from.to_oklch().l;
    let end = if scheme == Scheme::Dark { 0.0 } else { 1.0 };
    let (mut near, mut far) = (start, end);
    for _ in 0..40 {
        let middle = near.midpoint(far);
        if scale.at(middle).contrast_ratio(from) < ratio {
            near = middle;
        } else {
            far = middle;
        }
    }
    scale.at(far)
}

/// The contrast of `theme`'s deepest surface with its background: what rule 3 keeps.
pub fn deepest_contrast(theme: &ThemePalettes) -> f32 {
    Color::from_hex(theme.deepest).contrast_ratio(theme.background())
}

/// The neutral palette made for a chosen background (rules 2 to 4), and what the background
/// became.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Neutral {
    /// Steps 25 to 1000.
    pub steps: [Color; 16],
    /// Where the chosen color went.
    pub fit: BackgroundFit,
}

/// Rules 2 to 4: the neutral palette of `theme` for the chosen background `chosen`.
///
/// - Rule 2: a background in the mid-tones moves to the nearest readable lightness.
/// - Rule 3: the deepest surface (25) lies at the theme's own contrast from the background, on
///   the side away from the text. Where the end of the scale is nearer, it is that end, while it
///   keeps [`SURFACE_APART`]. Nearer still, the chosen color is the deepest surface, and the
///   background lies at the theme's contrast from it.
/// - Rule 4: steps 75 to 1000 at the theme's contrasts. When the background caps the contrast
///   below step 900's, the text steps share what is left above step 500, in log contrast, and
///   step 900 takes the strongest color the background allows.
pub fn neutral(theme: &ThemePalettes, chosen: Color) -> Neutral {
    let readable = readable(chosen, theme.text_room());
    let mut fit = if readable == chosen.with_alpha(1.0) {
        BackgroundFit::Exact
    } else {
        BackgroundFit::Moved(readable)
    };
    let scheme = Scheme::of(readable);
    let scale = ColorScale::from_seed(readable);
    let near_end = scale.at(if scheme == Scheme::Dark { 0.0 } else { 1.0 });
    let room = readable.contrast_ratio(near_end);
    let wanted = deepest_contrast(theme);
    let (deepest, background) = if room >= wanted {
        (toward(&scale, readable, wanted, scheme), readable)
    } else if room >= SURFACE_APART {
        (near_end, readable)
    } else {
        fit = BackgroundFit::Deepest;
        (readable, scale.with_contrast(readable, wanted))
    };
    let far_end = scale.at(if scheme == Scheme::Dark { 1.0 } else { 0.0 });
    let contrasts = compressed(theme.neutral_contrasts, background.contrast_ratio(far_end));
    let mut steps = contrasts.map(|ratio| scale.with_contrast(background, ratio));
    steps[0] = deepest;
    steps[1] = background;
    hold_white(
        scheme,
        &scale,
        background,
        &mut steps[2..=LAST_NEUTRAL_FILL],
    );
    Neutral { steps, fit }
}

/// Rule 4: `contrasts` with the text steps sharing, in log contrast, what lies between the last
/// step below `reach` and `reach`, when `reach` is below step 900's contrast. Step 1000, the
/// end of the scale, stays.
pub fn compressed(contrasts: [f32; 16], reach: f32) -> [f32; 16] {
    let top = *TEXT_STEPS.end();
    if contrasts[top] <= reach {
        return contrasts;
    }
    // The knee: step 500, or lower when the background caps even it.
    let knee = (1..*TEXT_STEPS.start())
        .rev()
        .find(|&index| contrasts[index] < reach)
        .unwrap_or(1);
    let (base, span) = (
        contrasts[knee].ln(),
        contrasts[top].ln() - contrasts[knee].ln(),
    );
    let share = if span > 0.0 {
        (reach.ln() - base) / span
    } else {
        0.0
    };
    let mut result = contrasts;
    for index in knee + 1..=top {
        result[index] = (base + (contrasts[index].ln() - base) * share).exp();
    }
    result
}

/// White content on the fills of a dark scheme: each step of `fills` that white does not read
/// on at [`WHITE_CONTENT`] is darkened until it does, as long as it then keeps [`fill_floor`]
/// from `background`; past that, it keeps its contrast and its content turns dark (rule 9).
pub fn hold_white(scheme: Scheme, scale: &ColorScale, background: Color, fills: &mut [Color]) {
    if scheme != Scheme::Dark {
        return;
    }
    for step in fills {
        if Color::WHITE.contrast_ratio(*step) >= WHITE_CONTENT {
            continue;
        }
        let held = scale.with_contrast(Color::WHITE, WHITE_CONTENT);
        if held.contrast_ratio(background) >= fill_floor() {
            *step = held;
        }
    }
}

/// Rules 5 and 7: the palette of `seed`'s scale for `background`, each step at its contrast,
/// the fills of a dark scheme held for white content ([`hold_white`]): what a hue made again
/// for a new background, or a role derived from it, takes.
pub fn made(seed: Color, background: Color) -> [Color; 16] {
    let scheme = Scheme::of(background);
    let scale = ColorScale::from_seed(seed.with_alpha(1.0));
    let mut steps = scale.steps(background, &scheme.hue_contrasts());
    hold_white(scheme, &scale, background, &mut steps[..=LAST_HUE_FILL]);
    steps
}

/// A palette made from a chosen color (rule 6), and whether the color had to move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pinned {
    /// The 16 steps, the fill step the chosen color.
    pub steps: [Color; 16],
    /// The fill, when the chosen color was moved to the fill step's own contrast.
    pub moved: Option<Color>,
}

/// Rule 6: the palette whose step `fill` is `chosen`, for `background`. `contrasts` are the
/// steps' contrasts with the background (the scheme's hue contrasts).
///
/// - A chosen color at least as far from the background as the fill step's own contrast is that
///   step exactly. A nearer one, or one on the background's side, moves along its scale to that
///   contrast: what the theme's own fills hold against every surface they lie on, they hold.
/// - Steps up to the fill keep their place in log contrast between 1:1 and the fill.
/// - Steps above keep their distance from it, compressed only where the scale ends sooner.
pub fn pinned(chosen: Color, background: Color, contrasts: &[f32; 16], fill: usize) -> Pinned {
    let chosen = chosen.with_alpha(1.0);
    let scheme = Scheme::of(background);
    let scale = ColorScale::from_seed(chosen);
    let far = match scheme {
        Scheme::Dark => chosen.relative_luminance() > background.relative_luminance(),
        Scheme::Light => chosen.relative_luminance() < background.relative_luminance(),
    };
    // A 0.5 % tolerance: the theme's own fill, picked from its 8-bit value, stays exact.
    let floor = contrasts[fill];
    let (fill_color, moved) = if far && chosen.contrast_ratio(background) >= floor * 0.995 {
        (chosen, None)
    } else {
        let moved = scale.with_contrast(background, floor);
        (moved, Some(moved))
    };
    let reach = background.contrast_ratio(scale.at(if scheme == Scheme::Dark { 1.0 } else { 0.0 }));
    let ratio = fill_color.contrast_ratio(background).ln();
    let own = contrasts[fill].ln();
    let below = if own > 0.0 { ratio / own } else { 1.0 };
    let span = contrasts[15].ln() - own;
    let above = if span > 0.0 {
        ((reach.ln() - ratio) / span).min(1.0)
    } else {
        1.0
    };
    let mut steps = [fill_color; 16];
    for (index, step) in steps.iter_mut().enumerate() {
        let log = contrasts[index].ln();
        let target = if index <= fill {
            log * below
        } else {
            ratio + (log - own) * above
        };
        if index != fill {
            *step = scale.with_contrast(background, target.exp());
        }
    }
    Pinned { steps, moved }
}

/// Rule 7: the accent made from the background, in the two background modes: the hue of a hued
/// background with the theme's own accent's lightness and chroma; `None` for a gray background,
/// which keeps the theme's accent.
pub fn accent_from(background: Color, own: Color) -> Option<Color> {
    let (background, own) = (background.to_oklch(), own.to_oklch());
    (background.c >= HUED).then(|| Oklch::new(own.l, own.c, background.h).to_color())
}

/// Rule 7: a role's own color turned toward the hue of a hued background by
/// [`HARMONY_SHARE`] of their difference, at most [`HARMONY_LIMIT`]; `None` when it does not
/// turn (a gray background, the same hue).
pub fn harmonized(own: Color, background: Color) -> Option<Color> {
    let (lch, background) = (own.to_oklch(), background.to_oklch());
    if background.c < HUED || lch.c < HUED {
        return None;
    }
    let difference = (background.h - lch.h + 540.0).rem_euclid(360.0) - 180.0;
    let turn = (difference * HARMONY_SHARE).clamp(-HARMONY_LIMIT, HARMONY_LIMIT);
    (turn.abs() >= 0.5)
        .then(|| Oklch::new(lch.l, lch.c, (lch.h + turn).rem_euclid(360.0)).to_color())
}

/// The hover and pressed steps of a role's fills, and their content (rules 8 and 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FillStates {
    /// The step of the hover fill.
    pub hover: usize,
    /// The step of the pressed fill.
    pub pressed: usize,
    /// White content (`true`) or black.
    pub white: bool,
}

/// Rules 8 and 9: the states and the content of fills whose default is step `default`.
///
/// States lie at `offsets` steps from the default (hover, pressed): the theme's own (dark
/// themes one and two steps toward the background, light ones away from it, high contrast one
/// each way), or, `flipped`, their opposite for a background of the other scheme. The
/// candidates are those offsets and their opposite, the theme's first (the opposite first when
/// `flipped`), each with white then black content, when they stay in the palette.
///
/// A candidate *reads* when its content reaches `content` on the default, hover and pressed
/// fills. It *shows its states* when the pressed fill lies [`PRESSED_APART`] or more from the
/// default. The first that reads and shows its states wins; else the first that reads; else
/// the one whose content reads best on its worst fill.
pub fn fill_states(
    steps: &[Color; 16],
    default: usize,
    offsets: (isize, isize),
    flipped: bool,
    content: f32,
) -> FillStates {
    let at = |offset: isize| {
        default
            .checked_add_signed(offset)
            .filter(|&index| index <= 15 && index != default)
    };
    let place = |(hover, pressed): (isize, isize)| Some((at(hover)?, at(pressed)?));
    let opposite = (-offsets.0, -offsets.1);
    let directions = if flipped {
        [place(opposite), place(offsets)]
    } else {
        [place(offsets), place(opposite)]
    };
    let mut best: Option<(u8, f32, FillStates)> = None;
    for (hover, pressed) in directions.into_iter().flatten() {
        let shows = steps[default].contrast_ratio(steps[pressed]) >= PRESSED_APART;
        for white in [true, false] {
            let ink = if white { Color::WHITE } else { Color::BLACK };
            let worst = [default, hover, pressed]
                .map(|index| ink.contrast_ratio(steps[index]))
                .into_iter()
                .fold(f32::INFINITY, f32::min);
            let reads = worst >= content;
            let tier = u8::from(reads) + u8::from(reads && shows);
            let candidate = FillStates {
                hover,
                pressed,
                white,
            };
            let better = match best {
                None => true,
                Some((best_tier, best_worst, _)) => {
                    tier > best_tier || (tier == 0 && best_tier == 0 && worst > best_worst)
                }
            };
            if better {
                best = Some((tier, worst, candidate));
            }
        }
    }
    let fallback = || FillStates {
        hover: at(offsets.0).unwrap_or(default),
        pressed: at(offsets.1).unwrap_or(default),
        white: true,
    };
    best.map_or_else(fallback, |(_, _, states)| states)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::{DARK_HUE_CONTRASTS, NEUTRAL_CONTRASTS, THEMES};

    #[test]
    fn readable_backgrounds_stay_and_mid_tones_move_to_the_nearer_bound() {
        for kept in [0x13_13_13, 0x30_30_30, 0xF8_F5_F3, 0xC8_C8_C8] {
            let color = Color::from_hex(kept);
            assert_eq!(readable(color, TEXT_ROOM), color, "#{kept:06X}");
        }
        for (mid, dark) in [(0x50_50_50, true), (0x70_70_70, true), (0xA0_A0_A0, false)] {
            let moved = readable(Color::from_hex(mid), TEXT_ROOM);
            let room = Color::WHITE
                .contrast_ratio(moved)
                .max(Color::BLACK.contrast_ratio(moved));
            assert!(room >= TEXT_ROOM - 0.05, "#{mid:06X}: {room}");
            assert_eq!(Scheme::of(moved) == Scheme::Dark, dark, "#{mid:06X}");
        }
    }

    #[test]
    fn text_steps_compress_only_when_the_background_caps_them() {
        assert_eq!(compressed(NEUTRAL_CONTRASTS, 18.0), NEUTRAL_CONTRASTS);
        let tight = compressed(NEUTRAL_CONTRASTS, TEXT_ROOM);
        assert_eq!(
            tight[..=10],
            NEUTRAL_CONTRASTS[..=10],
            "surfaces and borders keep theirs"
        );
        assert!((tight[14] - TEXT_ROOM).abs() < 1e-3, "{}", tight[14]);
        assert!(
            tight[10..=14].windows(2).all(|pair| pair[0] < pair[1]),
            "{tight:?}"
        );
    }

    #[test]
    fn a_pinned_palette_holds_its_color_in_contrast_order() {
        let background = Color::from_hex(0x13_13_13);
        for chosen in [0x3D_65_FC, 0xFF_88_00, 0xE0_F0_FF] {
            let chosen = Color::from_hex(chosen);
            let made = pinned(chosen, background, &DARK_HUE_CONTRASTS, 8);
            assert_eq!(made.moved, None);
            assert_eq!(made.steps[8], chosen);
            let ratios = made.steps.map(|step| step.contrast_ratio(background));
            assert!(
                ratios.windows(2).all(|pair| pair[0] <= pair[1] + 1e-3),
                "{ratios:?}"
            );
        }
        // At the step's own contrast, the steps are the scale's own.
        let scale = ColorScale::from_seed(Color::from_hex(0x3D_65_FC));
        let own = scale.with_contrast(background, DARK_HUE_CONTRASTS[8]);
        let made = pinned(own, background, &DARK_HUE_CONTRASTS, 8);
        for (index, step) in made.steps.iter().enumerate().take(8) {
            let ratio = step.contrast_ratio(background);
            let target = DARK_HUE_CONTRASTS[index];
            assert!(
                (ratio - target).abs() <= target * 0.02,
                "{index}: {ratio} {target}"
            );
        }
    }

    #[test]
    fn roles_turn_toward_a_hued_background_by_half_and_fifteen_degrees_at_most() {
        let red = Oklch::new(0.6, 0.2, 25.0).to_color();
        assert_eq!(harmonized(red, Color::from_hex(0x30_30_30)), None, "a gray");
        let near = harmonized(red, Oklch::new(0.2, 0.05, 45.0).to_color()).unwrap();
        assert!(
            (near.to_oklch().h - 35.0).abs() < 1.0,
            "{}",
            near.to_oklch().h
        );
        let far = harmonized(red, Oklch::new(0.2, 0.05, 200.0).to_color()).unwrap();
        let turn = (far.to_oklch().h - 25.0 + 540.0).rem_euclid(360.0) - 180.0;
        assert!((turn.abs() - HARMONY_LIMIT).abs() < 1.0, "{turn}");
    }

    #[test]
    fn the_built_in_dark_accent_keeps_its_states_and_white_content() {
        let dark = THEMES[0];
        let steps = dark.hue("blue").unwrap();
        let states = fill_states(&steps, 8, (-1, -2), false, CONTENT);
        assert_eq!(
            states,
            FillStates {
                hover: 7,
                pressed: 6,
                white: true
            }
        );
        // A fill close to the background has no room toward it: the states turn away, where
        // white content still reads.
        let background = dark.background();
        let scale = ColorScale::from_seed(Color::from_hex(0x2A_3A_70));
        let ratios = [
            1.02, 1.04, 1.06, 1.08, 1.1, 1.15, 1.2, 1.25, 1.3, 2.0, 2.6, 3.2, 4.0, 5.0, 6.0, 7.0,
        ];
        let steps = scale.steps(background, &ratios);
        let states = fill_states(&steps, 8, (-1, -2), false, CONTENT);
        assert_eq!((states.hover, states.pressed, states.white), (9, 10, true));
    }
}

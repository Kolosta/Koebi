//! Themes generated from chosen colors ([ADR 0027](../../../docs/adr/0027-themes-generated-from-chosen-colors.md)):
//! a background, an accent or a palette's color becomes every step, alias and content color
//! of a theme, by the rules of [`rules`], written as overrides of that theme.
//!
//! - **Inputs** are tokens: [`primitive::COLOR_MODE`] ([`Mode`]) and the palettes' sources. A
//!   source overridden in the theme is a *chosen* color; the neutral palette's is the
//!   background.
//! - **Outputs** are overrides of the theme: palette steps, the structure's aliases, the
//!   content on each role's fills. [`plan`] computes them from the inputs alone, without
//!   writing. A change ([`regenerate`]) plans before and after it, writes what the new plan
//!   holds, and removes what only the former held and nobody changed since. A hand edit of a
//!   generated token stays until the generator writes that token again.
//! - An output equal to the theme's own definition leaves no override: a theme with nothing
//!   chosen holds none.
//!
//! `docs/design-system/color.md` (*Generated themes*) states the rules and their constants.

pub mod rules;
mod structure;

pub use rules::BackgroundFit;
pub use structure::{LIGHT, ROLE_FILLS, RoleFills};

use std::collections::HashMap;

use koebi_core::Color;
use koebi_design::value::{Definition, Value};
use koebi_design::{DesignSystem, Layers, Scope, ThemeId, TokenId, View};

use crate::palettes::{
    self, Editable, Family, NEUTRAL_CONTRASTS, PaletteTokens, Scheme, ThemePalettes,
};
use crate::{primitive, semantic};

/// How much of a theme the chosen colors make ([`primitive::COLOR_MODE`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Every palette from its own color: a chosen one, or the theme's.
    Palettes,
    /// Every palette from the background: the accent takes its hue, the other roles lean to it.
    Background,
    /// The background and the accent; the other roles lean to the background's hue.
    BackgroundAccent,
}

impl Mode {
    /// Every mode, in the order of their values.
    pub const ALL: [Self; 3] = [Self::Palettes, Self::Background, Self::BackgroundAccent];

    /// The mode of `theme`: its own, or the user's.
    pub fn of(design: &DesignSystem, theme: ThemeId) -> Self {
        let value = design
            .view(theme, Layers::THEME | Layers::THEME_OVERRIDES)
            .and_then(|view| view.value(primitive::COLOR_MODE.id()));
        match value {
            Some(Value::Integer(1)) => Self::Background,
            Some(Value::Integer(2)) => Self::BackgroundAccent,
            _ => Self::Palettes,
        }
    }

    /// Its value in [`primitive::COLOR_MODE`].
    pub const fn value(self) -> i32 {
        match self {
            Self::Palettes => 0,
            Self::Background => 1,
            Self::BackgroundAccent => 2,
        }
    }

    /// Whether `palette`'s chosen color counts in this mode: every one in
    /// [`Mode::Palettes`], the background and the accent in [`Mode::BackgroundAccent`], the
    /// background alone in [`Mode::Background`]. Hue palettes count in every mode.
    pub fn takes(self, palette: &Editable) -> bool {
        match (self, palette.family) {
            (Self::Palettes, _) | (_, Family::Neutral | Family::Hue) => true,
            (Self::BackgroundAccent, Family::Role) => palette.key == "accent",
            (Self::Background, Family::Role) => false,
        }
    }
}

/// What the generator could not do as asked, for the settings to say.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// What the chosen background became; `None` without a chosen background.
    pub background: Option<BackgroundFit>,
    /// The palettes, by key, whose chosen color moved to its fill step's own contrast with the
    /// background (rule 6), with the fill they show.
    pub moved: Vec<(&'static str, Color)>,
}

/// The overrides a theme's inputs make, and what could not be honored.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    /// The definition of each output token, sorted by token.
    outputs: Vec<(TokenId, Definition)>,
    /// What the settings say about it.
    pub report: Report,
}

impl Plan {
    /// The definition the plan gives `token`, if it writes it.
    fn get(&self, token: TokenId) -> Option<&Definition> {
        self.outputs
            .binary_search_by_key(&token, |(id, _)| *id)
            .ok()
            .map(|index| &self.outputs[index].1)
    }

    /// How many tokens the plan writes.
    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    /// `true` when the plan writes nothing: the theme's own colors.
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }
}

/// The palettes' data for `theme`: the built-in theme's, or, for a theme imported or created at
/// run time, what its background and deepest surface give.
fn data_of(design: &DesignSystem, theme: ThemeId, own: &View<'_>) -> ThemePalettes {
    if let Some(data) = design
        .theme(theme)
        .and_then(|info| palettes::theme(info.key()))
    {
        return *data;
    }
    let color = |token: TokenId| match own.value(token) {
        Some(Value::Color(color)) => color,
        _ => Color::BLACK,
    };
    let hex = |color: Color| {
        let [r, g, b, _] = color.to_rgba8();
        u32::from_be_bytes([0, r, g, b])
    };
    let background = color(primitive::COLOR_GRAY_50.id());
    ThemePalettes {
        theme: "",
        scheme: Scheme::of(background),
        deepest: hex(color(primitive::COLOR_GRAY_25.id())),
        background: hex(background),
        neutral_keys: &[],
        neutral_contrasts: NEUTRAL_CONTRASTS,
        chroma: 1.0,
        seeds: &[],
        high_contrast: false,
    }
}

/// The color `palette`'s source holds as a chosen color of `theme`: its override in the theme.
fn chosen(design: &DesignSystem, theme: ThemeId, tokens: &PaletteTokens) -> Option<Color> {
    match design.override_of(Scope::Theme(theme), tokens.source) {
        Some(Definition::Value(Value::Color(color))) => Some(color.with_alpha(1.0)),
        _ => None,
    }
}

/// The plan of `theme`'s current inputs: every output, computed without writing (see the
/// module).
///
/// # Errors
///
/// When the theme is unknown, or a palette's token is missing from the schema.
pub fn plan(design: &DesignSystem, theme: ThemeId) -> koebi_design::Result<Plan> {
    let unknown = || koebi_design::Error::UnknownTheme(format!("{theme:?}"));
    let own = design.view(theme, Layers::THEME).ok_or_else(unknown)?;
    let base = design.view(theme, Layers::NONE).ok_or_else(unknown)?;
    let data = data_of(design, theme, &own);
    let mode = Mode::of(design, theme);
    let mut palettes = Vec::new();
    for palette in palettes::editable() {
        let tokens = palette
            .tokens(design)
            .ok_or_else(|| koebi_design::Error::UnknownToken(palette.source_path()))?;
        palettes.push((palette, tokens));
    }
    let mut values: HashMap<TokenId, Color> = HashMap::new();
    let mut report = Report::default();
    let own_color = |token: TokenId| match own.value(token) {
        Some(Value::Color(color)) => color,
        _ => Color::BLACK,
    };

    // Rules 2 to 4: the neutral palette and the background.
    let neutral = palettes
        .iter()
        .find(|(palette, _)| palette.family == Family::Neutral)
        .map(|(_, tokens)| *tokens);
    let chosen_background = neutral.and_then(|tokens| chosen(design, theme, &tokens));
    let background = match (chosen_background, neutral) {
        (Some(color), Some(tokens)) => {
            let made = rules::neutral(&data, color);
            report.background = Some(made.fit);
            for (token, step) in tokens.steps.into_iter().zip(made.steps) {
                values.insert(token, step);
            }
            made.steps[1]
        }
        _ => own_color(primitive::COLOR_GRAY_50.id()),
    };
    let scheme = Scheme::of(background);
    let contrasts = scheme.hue_contrasts();

    // Rules 5 and 6: the hues, made again for a new background or from a chosen color.
    for (palette, tokens) in palettes
        .iter()
        .filter(|(palette, _)| palette.family == Family::Hue)
    {
        let steps = match chosen(design, theme, tokens) {
            Some(color) => {
                let made = rules::pinned(color, background, &contrasts, 8);
                if let Some(moved) = made.moved {
                    report.moved.push((palette.key, moved));
                }
                Some(made.steps)
            }
            None if chosen_background.is_some() => data.hue_on(palette.key, background),
            None => None,
        };
        for (token, step) in tokens.steps.into_iter().zip(steps.into_iter().flatten()) {
            values.insert(token, step);
        }
    }

    // Rules 6 and 7: the roles, chosen, derived from the background, or following their hue.
    for (palette, tokens) in palettes
        .iter()
        .filter(|(palette, _)| palette.family == Family::Role)
    {
        let fills = ROLE_FILLS.iter().find(|fills| fills.key == palette.key);
        let fill = fills
            .and_then(|fills| own.definition(fills.default)?.alias_target())
            .and_then(|target| tokens.steps.iter().position(|&step| step == target))
            .unwrap_or(8);
        let own_source = own_color(tokens.source);
        let steps = match chosen(design, theme, tokens).filter(|_| mode.takes(palette)) {
            Some(color) => {
                let made = rules::pinned(color, background, &contrasts, fill);
                if let Some(moved) = made.moved {
                    report.moved.push((palette.key, moved));
                }
                Some(made.steps)
            }
            None => match mode {
                Mode::Palettes => None,
                Mode::Background if palette.key == "accent" => {
                    rules::accent_from(background, own_source)
                        .map(|seed| rules::made(seed, background))
                }
                Mode::BackgroundAccent if palette.key == "accent" => None,
                Mode::Background | Mode::BackgroundAccent => {
                    rules::harmonized(own_source, background)
                        .map(|seed| rules::made(seed, background))
                }
            },
        };
        if let Some(steps) = steps {
            for (token, step) in tokens.steps.into_iter().zip(steps) {
                values.insert(token, step);
            }
        }
    }

    // Stored as a theme file or a saved state stores them, in 8 bits: what is shown is what is
    // saved, exported and restored, whatever the platform's last bits of floating point.
    let mut outputs: HashMap<TokenId, Definition> = values
        .iter()
        .map(|(&token, &color)| (token, Definition::Value(Value::Color(stored(color)))))
        .collect();
    for moved in &mut report.moved {
        moved.1 = stored(moved.1);
    }
    if let Some(BackgroundFit::Moved(color)) = &mut report.background {
        *color = stored(*color);
    }

    // Rule 10: the other scheme's structure when the background changes scheme.
    if scheme != data.scheme {
        for (token, light) in LIGHT {
            let target = match scheme {
                Scheme::Light => Some(light),
                Scheme::Dark => base.definition(token).and_then(Definition::alias_target),
            };
            if let Some(target) = target {
                outputs.insert(token, Definition::Alias(target));
            }
        }
    }

    // What a token shows in the generated theme: its output, or the theme's own definition.
    let resolve = |outputs: &HashMap<TokenId, Definition>, token: TokenId| -> Option<Color> {
        let mut token = token;
        for _ in 0..16 {
            let definition = outputs.get(&token).or_else(|| own.definition(token));
            match definition {
                Some(Definition::Alias(target)) => token = *target,
                Some(Definition::Value(Value::Color(color))) => return Some(*color),
                Some(Definition::Value(_)) => return None,
                None => {
                    return match own.value(token) {
                        Some(Value::Color(color)) => Some(color),
                        _ => None,
                    };
                }
            }
        }
        None
    };
    let target = |outputs: &HashMap<TokenId, Definition>, token: TokenId| {
        outputs
            .get(&token)
            .or_else(|| own.definition(token))
            .and_then(Definition::alias_target)
    };

    // Rules 8 and 9: states and content of every role whose steps changed, by its own palette
    // or by the hue it aliases.
    let unchanged = HashMap::new();
    for fills in &ROLE_FILLS {
        let steps = fills
            .steps
            .map(|step| resolve(&outputs, step).unwrap_or(Color::TRANSPARENT));
        let before = fills
            .steps
            .map(|step| resolve(&unchanged, step).unwrap_or(Color::TRANSPARENT));
        if steps.map(Color::to_rgba8) == before.map(Color::to_rgba8) {
            continue;
        }
        let index = |token: TokenId| {
            target(&outputs, token).and_then(|target| fills.steps.iter().position(|&s| s == target))
        };
        let own_index = |token: TokenId| {
            own.definition(token)
                .and_then(Definition::alias_target)
                .and_then(|target| fills.steps.iter().position(|&s| s == target))
        };
        let offset = |state: Option<&TokenId>| -> Option<isize> {
            let state = own_index(*state?)?;
            let default = own_index(fills.default)?;
            Some(isize::try_from(state).ok()? - isize::try_from(default).ok()?)
        };
        if let (Some(default), Some(hover), Some(pressed)) = (
            index(fills.default),
            offset(fills.hover.first()),
            offset(fills.pressed.first()),
        ) {
            let states = rules::fill_states(
                &steps,
                default,
                (hover, pressed),
                scheme != data.scheme,
                data.content(),
            );
            for &token in fills.hover {
                outputs.insert(token, Definition::Alias(fills.steps[states.hover]));
            }
            for &token in fills.pressed {
                outputs.insert(token, Definition::Alias(fills.steps[states.pressed]));
            }
            let content = if states.white {
                semantic::STATIC_TEXT_COLOR_WHITE.id()
            } else {
                semantic::STATIC_TEXT_COLOR_BLACK.id()
            };
            outputs.insert(fills.content, Definition::Alias(content));
        }
    }

    // An output equal to the theme's own definition is no override.
    let mut outputs: Vec<(TokenId, Definition)> = outputs
        .into_iter()
        .filter(|(token, definition)| !same_as_own(&own, *token, definition))
        .collect();
    outputs.sort_by_key(|(token, _)| *token);
    Ok(Plan { outputs, report })
}

/// `color` as tokens store it: 8 bits a channel.
fn stored(color: Color) -> Color {
    let [r, g, b, a] = color.to_rgba8();
    Color::from_rgba8(r, g, b, a)
}

/// Whether `definition` is what `token` already holds in the theme itself.
fn same_as_own(own: &View<'_>, token: TokenId, definition: &Definition) -> bool {
    match (definition, own.definition(token)) {
        (Definition::Alias(a), Some(Definition::Alias(b))) => a == b,
        (Definition::Value(value), _) => own.value(token).is_some_and(|own| own.same_as(value)),
        _ => false,
    }
}

/// Writes `after` as `theme`'s overrides, and removes those that only `before` held and that
/// nobody changed since.
///
/// # Errors
///
/// When a definition is refused ([`koebi_design::Error`]).
fn apply(
    design: &mut DesignSystem,
    theme: ThemeId,
    before: &Plan,
    after: &Plan,
) -> koebi_design::Result<()> {
    let scope = Scope::Theme(theme);
    for (token, definition) in &before.outputs {
        let untouched = design
            .override_of(scope, *token)
            .is_some_and(|current| same_definition(current, definition));
        if after.get(*token).is_none() && untouched {
            design.reset(scope, *token)?;
        }
    }
    for (token, definition) in &after.outputs {
        let current = design.override_of(scope, *token);
        if !current.is_some_and(|current| same_definition(current, definition)) {
            design.set_override(scope, *token, definition.clone())?;
        }
    }
    Ok(())
}

/// Two definitions that show the same: the same alias, or values equal once stored.
fn same_definition(a: &Definition, b: &Definition) -> bool {
    match (a, b) {
        (Definition::Alias(a), Definition::Alias(b)) => a == b,
        (Definition::Value(a), Definition::Value(b)) => a.same_as(b),
        _ => false,
    }
}

/// Applies `change` to `theme`'s inputs (a source, the mode), then makes the theme again: the
/// outputs of the plan after the change are written, those of the plan before it that the
/// change no longer asks for are removed (see the module). Returns what could not be honored.
///
/// # Errors
///
/// When `change` fails, or a definition is refused ([`koebi_design::Error`]).
pub fn regenerate(
    design: &mut DesignSystem,
    theme: ThemeId,
    change: impl FnOnce(&mut DesignSystem) -> koebi_design::Result<()>,
) -> koebi_design::Result<Report> {
    let before = plan(design, theme)?;
    change(design)?;
    let after = plan(design, theme)?;
    apply(design, theme, &before, &after)?;
    Ok(after.report)
}

/// Sets `theme`'s mode and makes the theme again.
///
/// # Errors
///
/// When the theme is unknown ([`koebi_design::Error`]).
pub fn set_mode(
    design: &mut DesignSystem,
    theme: ThemeId,
    mode: Mode,
) -> koebi_design::Result<Report> {
    regenerate(design, theme, |design| {
        design.set_override(
            Scope::Theme(theme),
            primitive::COLOR_MODE.id(),
            Definition::Value(Value::Integer(mode.value())),
        )
    })
}

/// What could not be honored in `theme` as its inputs stand.
pub fn report(design: &DesignSystem, theme: ThemeId) -> Report {
    plan(design, theme)
        .map(|plan| plan.report)
        .unwrap_or_default()
}

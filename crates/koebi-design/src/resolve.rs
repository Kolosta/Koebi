//! [`Resolve`]: one way to read token values, whatever their source.

use crate::kind::Kind;
use crate::system::DesignSystem;
use crate::tier::Tier;
use crate::token::{TextStyle, Token, Typography};
use crate::view::View;

/// Reads resolved token values: the active theme ([`DesignSystem`]) or any [`View`].
///
/// Code that styles components takes `&impl Resolve`, so the same function styles the live
/// interface and previews of other themes:
///
/// ```
/// # use koebi_design::Resolve;
/// # use koebi_core::Color;
/// # mod tokens {
/// #     use koebi_design::define::*;
/// #     koebi_design::tokens! {
/// #         group PALETTE: Primitive, start = 0;
/// #         /// Blue.
/// #         COLOR_ACCENT = "color.accent": Color = hex("#5D89FF");
/// #         /// Corner radius.
/// #         RADIUS_CONTROL = "radius.control": Dimension = px(3.0);
/// #     }
/// #     pub const SCHEMA: koebi_design::schema::Schema = koebi_design::schema::Schema::new(
/// #         "example",
/// #         koebi_design::schema::ThemeInfo::new("base", "Base", "Base theme.", &[]),
/// #         &[PALETTE::GROUP],
/// #         &[],
/// #     );
/// # }
/// # static SCHEMA: koebi_design::schema::Schema = tokens::SCHEMA;
/// struct ButtonStyle {
///     background: Color,
///     corner_radius: f32,
/// }
///
/// impl ButtonStyle {
///     fn resolve(source: &impl Resolve) -> Self {
///         Self {
///             background: source.get(tokens::COLOR_ACCENT),
///             corner_radius: source.get(tokens::RADIUS_CONTROL),
///         }
///     }
/// }
///
/// let design = koebi_design::DesignSystem::new(&SCHEMA)?;
/// let live = ButtonStyle::resolve(&design);
/// let original = ButtonStyle::resolve(
///     &design.view(design.active_theme(), koebi_design::Layers::NONE).expect("live theme"),
/// );
/// assert_eq!(live.corner_radius, original.corner_radius);
/// # Ok::<(), koebi_design::Error>(())
/// ```
pub trait Resolve {
    /// The value of `token`.
    ///
    /// A token of another schema reads as the kind's fallback (magenta for colors).
    fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value;

    /// The resolved text style of a typography composite.
    fn typography<T: Tier>(&self, style: Typography<T>) -> TextStyle {
        TextStyle {
            font_family: self.get(style.font_family),
            font_size: self.get(style.font_size),
            font_weight: self.get(style.font_weight),
            line_height: self.get(style.line_height),
            letter_spacing: self.get(style.letter_spacing),
        }
    }
}

impl Resolve for DesignSystem {
    #[inline]
    fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value {
        DesignSystem::get(self, token)
    }
}

impl Resolve for View<'_> {
    #[inline]
    fn get<K: Kind, T: Tier>(&self, token: Token<K, T>) -> K::Value {
        View::get(self, token)
    }
}

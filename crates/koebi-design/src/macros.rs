//! Token table macros: [`tokens!`](crate::tokens), [`typography!`](crate::typography) and
//! [`theme!`](crate::theme).
//!
//! A table defines one Rust constant per token, documented by its `///` comment, plus a
//! hidden module named after the group that holds the static description of the tokens.
//! Everything is checked while compiling: the kind and tier of every alias by the type system,
//! paths and constant names by [`schema::assert_group`](crate::schema::assert_group).

/// Defines a group of tokens of one tier: one constant per token, plus the group description
/// used by [`Schema::new`](crate::schema::Schema::new).
///
/// Each line defines a token by a value (`=`) or by an alias (`->`):
///
/// ```text
/// CONSTANT = "path": Kind = value;     // primitive and component tokens
/// CONSTANT = "path": Kind -> TARGET;   // semantic and component tokens
/// ```
///
/// optionally followed by the values the token accepts (a [`Range`](crate::value::Range)):
///
/// | Constraint | Accepts |
/// |---|---|
/// | `range(min, max)`, `range(min, max, step)` | numbers from `min` to `max`; `step` is a hint for editors |
/// | `at_least(min)`, `at_most(max)` | numbers from `min` up, or up to `max` |
/// | `positive()` | numbers above zero |
/// | `steps(min, max, step)` | multiples of `step` from `min` to `max` |
/// | `alpha(min, max)`, `opaque()` | colors whose alpha lies in the range, or is 1 |
///
/// The constant is the path in upper case, `.` and `-` becoming `_`. The `///` comment is the
/// token's description. Values are built with the constructors of [`define`](crate::define):
/// `hex("#3B63FB")`, `px(4.0)`, `ms(150)`... The tier rules are checked while compiling:
/// primitive tokens hold values, semantic tokens alias primitive or semantic tokens, component
/// tokens hold a value or alias semantic or component tokens.
///
/// `start` is the position of the first token: `0` for the first group of a schema, the `END`
/// of the previous group otherwise. It is evaluated inside the group's hidden module, which
/// imports everything visible in the enclosing module, so `PALETTE::END` or
/// `crate::primitive::PALETTE::END` both work.
///
/// ```
/// mod primitive {
///     use koebi_design::define::*;
///     koebi_design::tokens! {
///         group PALETTE: Primitive, start = 0;
///         /// Blue, palette step 700.
///         COLOR_BLUE_700 = "color.blue.700": Color = hex("#5D89FF");
///         /// Corner radius scale, step 200.
///         RADIUS_200 = "radius.200": Dimension = px(4.0), range(0.0, 32.0);
///     }
/// }
/// mod semantic {
///     use super::primitive;
///     koebi_design::tokens! {
///         group INTENTS: Semantic, start = primitive::PALETTE::END;
///         /// Background of accent elements.
///         BACKGROUND_ACCENT_DEFAULT = "background.accent.default": Color -> primitive::COLOR_BLUE_700;
///     }
/// }
/// mod component {
///     use super::semantic;
///     use koebi_design::define::*;
///     koebi_design::tokens! {
///         group BUTTON: Component, start = semantic::INTENTS::END;
///         /// Button background.
///         BUTTON_BACKGROUND = "button.background": Color -> semantic::BACKGROUND_ACCENT_DEFAULT;
///         /// Button height: a component token may hold its own value.
///         BUTTON_HEIGHT = "button.height": Dimension = px(28.0), range(16.0, 64.0);
///     }
/// }
/// fn main() {
///     assert_eq!(semantic::BACKGROUND_ACCENT_DEFAULT.id().index(), 2);
///     assert_eq!(component::BUTTON_HEIGHT.id().index(), 4);
/// }
/// ```
///
/// Semantic tokens never hold values:
///
/// ```compile_fail
/// mod semantic {
///     use koebi_design::define::*;
///     koebi_design::tokens! {
///         group SHAPES: Semantic, start = 0;
///         /// Small corner radius.
///         RADIUS_S = "radius.s": Dimension = px(4.0);
///     }
/// }
/// fn main() {}
/// ```
#[macro_export]
macro_rules! tokens {
    (
        group $group:ident: $tier:ident, start = $start:expr;
        $(
            $(#[doc = $doc:literal])*
            $name:ident = $path:literal: $kind:ident $op:tt $definition:expr
            $(, $constraint:ident($($argument:expr),*))?;
        )*
    ) => {
        $crate::__token_group! {
            $group, $tier, $start;
            $(
                [$($doc)*] $name $path $kind [$op $definition]
                ($crate::__token_definition!($op, $kind, $tier, $definition))
                [$($constraint($($argument),*))?]
            )*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __token_definition {
    (=, $kind:ident, $tier:ident, $value:expr) => {
        $crate::kind::$kind::literal($value)
    };
    (->, $kind:ident, $tier:ident, $target:expr) => {
        $crate::value::Def::alias::<$crate::kind::$kind, $crate::tier::$tier, _>($target)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __token_group {
    (
        $group:ident, $tier:ident, $start:expr;
        $(
            [$($doc:literal)*] $name:ident $path:literal $kind:ident [$($shown:tt)*]
            ($def:expr) [$($range:tt)*]
        )*
    ) => {
        #[doc(hidden)]
        #[allow(
            non_snake_case,
            missing_docs,
            unreachable_pub,
            clippy::upper_case_acronyms,
            dead_code,
            reason = "a hidden module named after its group, holding generated items"
        )]
        pub mod $group {
            #[allow(unused_imports, reason = "the group sees the items of the enclosing module")]
            use super::*;

            #[allow(
                non_camel_case_types,
                reason = "the index variants are named after the token constants"
            )]
            #[repr(u16)]
            pub enum Index {
                $($name,)*
                __End,
            }

            pub const START: u16 = $start;
            pub const END: u16 = START + Index::__End as u16;
            pub const TOKENS: [$crate::schema::TokenInfo; Index::__End as usize] = [
                $(
                    $crate::schema::TokenInfo::new(
                        $path,
                        stringify!($name),
                        $crate::tier::TierId::$tier,
                        $crate::kind::KindId::$kind,
                        concat!($($doc),*),
                        $def,
                        $crate::__token_range!($($range)*),
                    ),
                )*
            ];
            pub const GROUP: $crate::schema::Group =
                $crate::schema::Group::new(stringify!($group), START, &TOKENS, &[]);
            const _: () = $crate::schema::assert_group(&GROUP);
        }

        $(
            $(#[doc = $doc])*
            #[doc = ""]
            #[doc = concat!(
                "`", stringify!($tier), " ", $path, ": ", stringify!($kind), " ",
                stringify!($($shown)*), "`"
            )]
            pub const $name: $crate::Token<$crate::kind::$kind, $crate::tier::$tier> =
                $crate::Token::from_index($group::START + $group::Index::$name as u16);
        )*
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __token_range {
    () => {
        ::core::option::Option::None
    };
    (range($min:expr, $max:expr)) => {
        ::core::option::Option::Some($crate::value::Range::new($min, $max, 0.0))
    };
    (range($min:expr, $max:expr, $step:expr)) => {
        ::core::option::Option::Some($crate::value::Range::new($min, $max, $step))
    };
    ($constraint:ident($($argument:expr),*)) => {
        ::core::option::Option::Some($crate::value::Range::$constraint($($argument),*))
    };
}

/// Defines typography composites: text styles whose five fields are tokens of their own.
///
/// Each composite `TEXT_BODY_M = "text.body.m" { ... }` creates the field tokens
/// `text.body.m.font-family`, `.font-size`, `.font-weight`, `.line-height` and
/// `.letter-spacing`, each aliasing the given token, and a
/// [`Typography`](crate::Typography) constant giving access to them.
///
/// ```
/// mod primitive {
///     use koebi_design::define::*;
///     koebi_design::tokens! {
///         group FONT: Primitive, start = 0;
///         /// Sans-serif font family.
///         FONT_FAMILY_SANS = "font.family.sans": FontFamily = family("Noto Sans");
///         /// Font size scale, step 60.
///         FONT_SIZE_60 = "font.size.60": Dimension = px(15.0);
///         /// Regular font weight.
///         FONT_WEIGHT_REGULAR = "font.weight.regular": FontWeight = weight(400);
///         /// Normal line height.
///         FONT_LINE_HEIGHT_NORMAL = "font.line-height.normal": Number = 1.5;
///         /// Normal letter spacing.
///         FONT_TRACKING_NORMAL = "font.tracking.normal": Number = 0.0;
///     }
/// }
/// use primitive::*;
/// koebi_design::typography! {
///     group TEXT: Semantic, start = primitive::FONT::END;
///     /// Body text, medium size.
///     TEXT_BODY_M = "text.body.m" {
///         font_family: FONT_FAMILY_SANS,
///         font_size: FONT_SIZE_60,
///         font_weight: FONT_WEIGHT_REGULAR,
///         line_height: FONT_LINE_HEIGHT_NORMAL,
///         letter_spacing: FONT_TRACKING_NORMAL,
///     }
/// }
/// fn main() {
///     assert_eq!(TEXT_BODY_M.font_size.id().index(), 6);
/// }
/// ```
#[macro_export]
macro_rules! typography {
    (
        group $group:ident: $tier:ident, start = $start:expr;
        $(
            $(#[doc = $doc:literal])*
            $name:ident = $path:literal {
                font_family: $family:expr,
                font_size: $size:expr,
                font_weight: $weight:expr,
                line_height: $line:expr,
                letter_spacing: $spacing:expr $(,)?
            }
        )*
    ) => {
        #[doc(hidden)]
        #[allow(
            non_snake_case,
            missing_docs,
            unreachable_pub,
            clippy::upper_case_acronyms,
            dead_code,
            reason = "a hidden module named after its group, holding generated items"
        )]
        pub mod $group {
            #[allow(unused_imports, reason = "the group sees the items of the enclosing module")]
            use super::*;

            #[allow(
                non_camel_case_types,
                reason = "the index variants are named after the token constants"
            )]
            #[repr(u16)]
            pub enum Index {
                $($name,)*
                __End,
            }

            pub const START: u16 = $start;
            pub const END: u16 = START + Index::__End as u16 * 5;
            pub const TOKENS: [$crate::schema::TokenInfo; Index::__End as usize * 5] = [
                $(
                    $crate::__typography_field!(
                        $tier, $path, "font-family", FontFamily, $family, "Font family.",
                        [$($doc)*]
                    ),
                    $crate::__typography_field!(
                        $tier, $path, "font-size", Dimension, $size, "Font size.", [$($doc)*]
                    ),
                    $crate::__typography_field!(
                        $tier, $path, "font-weight", FontWeight, $weight, "Font weight.",
                        [$($doc)*]
                    ),
                    $crate::__typography_field!(
                        $tier, $path, "line-height", Number, $line, "Line height.",
                        [$($doc)*]
                    ),
                    $crate::__typography_field!(
                        $tier, $path, "letter-spacing", Number, $spacing, "Letter spacing.",
                        [$($doc)*]
                    ),
                )*
            ];
            pub const COMPOSITES: [$crate::schema::CompositeInfo; Index::__End as usize] = [
                $(
                    $crate::schema::CompositeInfo::new(
                        $path,
                        concat!($($doc),*),
                        $crate::tier::TierId::$tier,
                        $crate::schema::CompositeKind::Typography,
                        START + Index::$name as u16 * 5,
                    ),
                )*
            ];
            pub const GROUP: $crate::schema::Group =
                $crate::schema::Group::new(stringify!($group), START, &TOKENS, &COMPOSITES);
            const _: () = $crate::schema::assert_group(&GROUP);
        }

        $(
            $(#[doc = $doc])*
            #[doc = ""]
            #[doc = concat!("`", stringify!($tier), " ", $path, ": typography`")]
            pub const $name: $crate::Typography<$crate::tier::$tier> =
                $crate::Typography::from_first($group::START + $group::Index::$name as u16 * 5);
        )*
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __typography_field {
    (
        $tier:ident, $path:literal, $segment:literal, $kind:ident, $target:expr,
        $label:literal, [$($doc:literal)*]
    ) => {
        $crate::schema::TokenInfo::new(
            concat!($path, ".", $segment),
            "",
            $crate::tier::TierId::$tier,
            $crate::kind::KindId::$kind,
            concat!($($doc,)* " ", $label),
            $crate::value::Def::alias::<$crate::kind::$kind, $crate::tier::$tier, _>($target),
            ::core::option::Option::None,
        )
    };
}

/// Defines themes: each lists the tokens it changes, as aliases, on top of the base theme.
///
/// ```
/// mod tokens {
///     use koebi_design::define::*;
///     koebi_design::tokens! {
///         group PALETTE: Primitive, start = 0;
///         /// Gray, palette step 50.
///         COLOR_GRAY_50 = "color.gray.50": Color = hex("#F8F8F8");
///         /// Gray, palette step 900.
///         COLOR_GRAY_900 = "color.gray.900": Color = hex("#131313");
///     }
///     koebi_design::tokens! {
///         group INTENTS: Semantic, start = PALETTE::END;
///         /// Base background of the application.
///         BACKGROUND_BASE_DEFAULT = "background.base.default": Color -> COLOR_GRAY_900;
///     }
///     koebi_design::theme! {
///         /// Light theme.
///         LIGHT = "light", "Light" {
///             BACKGROUND_BASE_DEFAULT => COLOR_GRAY_50;
///         }
///     }
/// }
/// fn main() {
///     assert_eq!(tokens::LIGHT.entries().len(), 1);
/// }
/// ```
#[macro_export]
macro_rules! theme {
    (
        $(
            $(#[doc = $doc:literal])*
            $name:ident = $key:literal, $label:literal {
                $($token:expr => $target:expr;)*
            }
        )*
    ) => {
        $(
            $(#[doc = $doc])*
            pub const $name: $crate::schema::ThemeInfo = $crate::schema::ThemeInfo::new(
                $key,
                $label,
                concat!($($doc),*),
                &[$($crate::schema::ThemeEntry::alias($token, $target),)*],
            );
        )*
    };
}

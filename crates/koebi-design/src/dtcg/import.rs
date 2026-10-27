//! Import from the Design Tokens format, with strict and complete validation.

use core::fmt;
use core::time::Duration;
use std::sync::Arc;

use koebi_core::{Color, LinearRgba, Oklab, Oklch};

use crate::check;
use crate::define;
use crate::dtcg::{EXTENSION, FORMAT};
use crate::error::{Error, Result};
use crate::json::{self, Json, JsonValue, Position};
use crate::kind::KindId;
use crate::schema::{CompositeInfo, TokenInfo};
use crate::system::DesignSystem;
use crate::theme::{NewTheme, ThemeId};
use crate::tier::TierId;
use crate::token::TokenId;
use crate::value::{CubicBezier, Curve, CurveKey, Definition, FontWeight, Value};

/// One problem found while importing, located in the source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportIssue {
    /// Line of the offending value, 1-based (0 when the problem concerns the whole file).
    pub line: u32,
    /// Column of the offending value, 1-based.
    pub column: u32,
    /// Token or group path, when known.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for ImportIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}, column {}: ", self.line, self.column)?;
        }
        if !self.path.is_empty() {
            write!(f, "{}: ", self.path)?;
        }
        f.write_str(&self.message)
    }
}

/// Reads a theme written by [`export_theme`](super::export_theme), or by hand, into a new
/// theme of `design`. The theme's key, name and description come from Koebi's extension at the
/// root of the file.
///
/// # Errors
///
/// [`Error::Import`] with every problem found: invalid JSON, unknown properties, tiers or
/// tokens, values of the wrong type or out of range, tier violations, a missing or taken theme
/// key, alias cycles. Nothing is created on error.
pub fn import_theme(design: &mut DesignSystem, text: &str) -> Result<ThemeId> {
    let document = json::parse(text).map_err(|error| {
        Error::Import(vec![ImportIssue {
            line: error.position.line,
            column: error.position.column,
            path: String::new(),
            message: error.message,
        }])
    })?;
    let mut importer = Importer {
        design,
        issues: Vec::new(),
        definitions: Vec::new(),
        identity: None,
    };
    importer.root(&document);
    let Importer {
        issues,
        definitions,
        identity,
        ..
    } = importer;
    let mut issues = issues;
    let Some(identity) = identity else {
        issues.push(ImportIssue {
            line: document.position.line,
            column: document.position.column,
            path: String::new(),
            message: format!(
                "missing theme identity: $extensions[\"{EXTENSION}\"].theme with key and name"
            ),
        });
        return Err(Error::Import(issues));
    };
    if !issues.is_empty() {
        return Err(Error::Import(issues));
    }
    design
        .create_theme(NewTheme {
            key: identity.key,
            name: identity.name,
            description: identity.description,
            definitions,
        })
        .map_err(|error| {
            Error::Import(vec![ImportIssue {
                line: 0,
                column: 0,
                path: String::new(),
                message: error.to_string(),
            }])
        })
}

struct Identity {
    key: String,
    name: String,
    description: String,
}

struct Importer<'a> {
    design: &'a DesignSystem,
    issues: Vec<ImportIssue>,
    definitions: Vec<(TokenId, Definition)>,
    identity: Option<Identity>,
}

/// A type of the Design Tokens format that Koebi reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenType {
    /// A type with a kind of its own.
    Kind(KindId),
    /// The typography composite.
    Typography,
}

fn token_type(name: &str) -> Option<TokenType> {
    Some(TokenType::Kind(match name {
        "color" => KindId::Color,
        "dimension" => KindId::Dimension,
        "number" => KindId::Number,
        "duration" => KindId::Duration,
        "cubicBezier" => KindId::CubicBezier,
        "fontWeight" => KindId::FontWeight,
        "fontFamily" => KindId::FontFamily,
        "curve" => KindId::Curve,
        "typography" => return Some(TokenType::Typography),
        _ => return None,
    }))
}

impl Importer<'_> {
    fn issue(&mut self, position: Position, path: &str, message: impl Into<String>) {
        self.issues.push(ImportIssue {
            line: position.line,
            column: position.column,
            path: path.to_owned(),
            message: message.into(),
        });
    }

    fn root(&mut self, document: &Json) {
        let Some(members) = document.as_object() else {
            self.issue(document.position, "", "the document must be an object");
            return;
        };
        for (name, value) in members {
            match name.as_str() {
                "$description" | "$schema" => self.expect_string(value, name),
                "$extensions" => self.root_extension(value),
                tier => match TierId::from_name(tier) {
                    Some(_) => self.group(value, tier, None),
                    None if tier.starts_with('$') => {
                        self.issue(value.position, "", format!("unknown property {tier}"));
                    }
                    None => self.issue(
                        value.position,
                        tier,
                        "unknown tier: groups start with primitive, semantic or component",
                    ),
                },
            }
        }
    }

    fn expect_string(&mut self, value: &Json, path: &str) {
        if value.as_str().is_none() {
            self.issue(
                value.position,
                path,
                format!("expected a string, found {}", value.type_name()),
            );
        }
    }

    fn root_extension(&mut self, extensions: &Json) {
        if extensions.as_object().is_none() {
            self.issue(extensions.position, "$extensions", "expected an object");
            return;
        }
        let Some(koebi) = extensions.get(EXTENSION) else {
            return;
        };
        if let Some(format) = koebi.get("format") {
            match format.as_f64() {
                Some(version) if version <= FORMAT => {}
                _ => self.issue(
                    format.position,
                    "format",
                    "unsupported version of the Koebi extension",
                ),
            }
        }
        let Some(theme) = koebi.get("theme") else {
            return;
        };
        let text = |key: &str| theme.get(key).and_then(Json::as_str).map(str::to_owned);
        match (text("key"), text("name")) {
            (Some(key), Some(name)) => {
                if let Some(problem) = check::theme_key_problem(&key) {
                    self.issue(theme.position, "theme.key", problem);
                }
                self.identity = Some(Identity {
                    key,
                    name,
                    description: text("description").unwrap_or_default(),
                });
            }
            _ => self.issue(
                theme.position,
                "theme",
                "the theme needs a key and a name (strings)",
            ),
        }
    }

    /// A group: a JSON object without `$value`.
    fn group(&mut self, node: &Json, path: &str, inherited: Option<&str>) {
        let Some(members) = node.as_object() else {
            self.issue(
                node.position,
                path,
                format!("expected a group or a token, found {}", node.type_name()),
            );
            return;
        };
        if node.get("$value").is_some() {
            self.token(node, path, inherited);
            return;
        }
        let mut group_type = inherited.map(str::to_owned);
        if let Some(value) = node.get("$type") {
            match value.as_str() {
                Some(name) if token_type(name).is_some() => group_type = Some(name.to_owned()),
                Some(name) => self.issue(value.position, path, format!("unsupported type {name}")),
                None => self.issue(value.position, path, "$type must be a string"),
            }
        }
        for (name, child) in members {
            match name.as_str() {
                "$type" | "$extensions" | "$deprecated" => {}
                "$description" => self.expect_string(child, path),
                other if other.starts_with('$') => {
                    self.issue(child.position, path, format!("unknown property {other}"));
                }
                segment => {
                    let child_path = format!("{path}.{segment}");
                    if check::path_problem(segment).is_some() || segment.contains('.') {
                        self.issue(
                            child.position,
                            &child_path,
                            "invalid name: use a-z, 0-9 and -",
                        );
                        continue;
                    }
                    self.group(child, &child_path, group_type.as_deref());
                }
            }
        }
    }

    /// A token: a JSON object with `$value`.
    fn token(&mut self, node: &Json, path: &str, inherited: Option<&str>) {
        let members = node.as_object().unwrap_or_default();
        for (name, value) in members {
            match name.as_str() {
                "$value" | "$type" | "$extensions" | "$deprecated" => {}
                "$description" => self.expect_string(value, path),
                other => self.issue(
                    value.position,
                    path,
                    format!("unexpected member {other}: a token holds $value, $type, $description and $extensions"),
                ),
            }
        }
        let type_name = if let Some(declared) = node.get("$type") {
            let Some(name) = declared.as_str() else {
                self.issue(declared.position, path, "$type must be a string");
                return;
            };
            Some(name.to_owned())
        } else {
            inherited.map(str::to_owned)
        };
        let Some(value) = node.get("$value") else {
            return;
        };
        if let Some(composite) = self.composite(path) {
            if let Some(name) = &type_name
                && name != composite.kind().name()
            {
                let message = format!("{path} is a {}, not a {name}", composite.kind().name());
                self.issue(node.position, path, message);
                return;
            }
            self.composite_value(value, path, composite);
            return;
        }
        let Some(token) = self.design.find(path) else {
            self.issue(node.position, path, "unknown token");
            return;
        };
        let Some(kind) = self.design.info(token).map(TokenInfo::kind) else {
            return;
        };
        if let Some(name) = &type_name {
            let matches = match token_type(name) {
                Some(TokenType::Kind(declared)) => {
                    declared == kind || (declared == KindId::Number && kind == KindId::Integer)
                }
                Some(TokenType::Typography) => false,
                None => {
                    self.issue(node.position, path, format!("unsupported type {name}"));
                    return;
                }
            };
            if !matches {
                let message = format!("{path} holds a {}, not a {name}", kind.name());
                self.issue(node.position, path, message);
                return;
            }
        }
        self.define(token, kind, value, path);
    }

    fn composite(&self, path: &str) -> Option<&'static CompositeInfo> {
        let (tier, rest) = path.split_once('.')?;
        self.design
            .schema()
            .composites()
            .find(|c| c.tier().name() == tier && c.path() == rest)
    }

    fn composite_value(&mut self, value: &Json, path: &str, composite: &CompositeInfo) {
        let Some(members) = value.as_object() else {
            self.issue(value.position, path, "a typography value is an object");
            return;
        };
        let fields = composite.kind().fields();
        for (key, _) in members {
            if !fields.iter().any(|field| field.key == key) {
                let known: Vec<_> = fields.iter().map(|field| field.key).collect();
                self.issue(
                    value.position,
                    path,
                    format!("unknown field {key} (expected {})", known.join(", ")),
                );
            }
        }
        for (offset, field) in fields.iter().enumerate() {
            let field_path = format!("{path}.{}", field.segment);
            let Some(field_value) = value.get(field.key) else {
                self.issue(value.position, path, format!("missing field {}", field.key));
                continue;
            };
            let Some(token) = u16::try_from(offset)
                .ok()
                .map(|offset| TokenId::from_index(composite.first().get() + offset))
            else {
                continue;
            };
            self.define(token, field.kind, field_value, &field_path);
        }
    }

    /// Parses a value or an alias for `token` and checks it against the schema.
    fn define(&mut self, token: TokenId, kind: KindId, value: &Json, path: &str) {
        let definition = if let Some(target) = value.as_str().and_then(alias_path) {
            let Some(target_token) = self.design.find(target) else {
                self.issue(
                    value.position,
                    path,
                    format!("unknown alias target {{{target}}}"),
                );
                return;
            };
            Definition::Alias(target_token)
        } else {
            match parse_value(kind, value) {
                Ok(value) => Definition::Value(value),
                Err(message) => {
                    self.issue(value.position, path, message);
                    return;
                }
            }
        };
        if self
            .definitions
            .iter()
            .any(|(existing, _)| *existing == token)
        {
            self.issue(value.position, path, "defined twice");
            return;
        }
        match self.design.check_definition(token, definition) {
            Ok(definition) => self.definitions.push((token, definition)),
            Err(error) => self.issue(value.position, path, error.to_string()),
        }
    }
}

/// The path inside `{...}`, if `text` is an alias.
fn alias_path(text: &str) -> Option<&str> {
    text.strip_prefix('{')?.strip_suffix('}')
}

fn parse_value(kind: KindId, value: &Json) -> core::result::Result<Value, String> {
    match kind {
        KindId::Color => parse_color(value).map(Value::Color),
        KindId::Dimension => {
            let (amount, unit) = amount_and_unit(value, "dimension")?;
            match unit {
                "px" => Ok(Value::Dimension(narrow(amount))),
                // The Design Tokens format's root size.
                "rem" => Ok(Value::Dimension(narrow(amount * 16.0))),
                other => Err(format!("unsupported dimension unit {other} (px or rem)")),
            }
        }
        KindId::Number => number(value).map(|n| Value::Number(narrow(n))),
        KindId::Integer => {
            let n = number(value)?;
            if n.fract() == 0.0 && n >= f64::from(i32::MIN) && n <= f64::from(i32::MAX) {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "checked above: an integer within the range of i32"
                )]
                Ok(Value::Integer(n as i32))
            } else {
                Err(format!("expected an integer, found {n}"))
            }
        }
        KindId::Duration => {
            let (amount, unit) = amount_and_unit(value, "duration")?;
            let seconds = match unit {
                "ms" => amount / 1000.0,
                "s" => amount,
                other => return Err(format!("unsupported duration unit {other} (ms or s)")),
            };
            Duration::try_from_secs_f64(seconds)
                .map(Value::Duration)
                .map_err(|_| "durations are finite and not negative".to_owned())
        }
        KindId::CubicBezier => {
            let items = value
                .as_array()
                .filter(|items| items.len() == 4)
                .ok_or_else(|| "a cubic Bézier is an array of four numbers".to_owned())?;
            let mut points = [0.0_f32; 4];
            for (point, item) in points.iter_mut().zip(items) {
                *point = narrow(number(item)?);
            }
            let curve = CubicBezier::new(points[0], points[1], points[2], points[3]);
            if curve.is_valid() {
                Ok(Value::CubicBezier(curve))
            } else {
                Err("cubic Bézier x coordinates must lie in 0..=1".to_owned())
            }
        }
        KindId::FontWeight => {
            let weight = if let Some(keyword) = value.as_str() {
                font_weight_keyword(keyword)
                    .ok_or_else(|| format!("unknown font weight {keyword}"))?
            } else {
                let n = number(value)?;
                if n.fract() != 0.0 || !(1.0..=1000.0).contains(&n) {
                    return Err(format!("font weights are integers in 1..=1000, found {n}"));
                }
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "checked above: an integer in 1..=1000"
                )]
                let weight = n as u16;
                weight
            };
            FontWeight::new(weight)
                .map(Value::FontWeight)
                .ok_or_else(|| "font weights lie in 1..=1000".to_owned())
        }
        KindId::Curve => parse_curve(value),
        KindId::FontFamily => {
            let name = match &value.value {
                JsonValue::String(name) => Some(name.as_str()),
                // A list of families: the first one, the others being fallbacks.
                JsonValue::Array(items) => items.first().and_then(Json::as_str),
                _ => None,
            };
            name.filter(|name| !name.trim().is_empty())
                .map(|name| Value::FontFamily(Arc::from(name)))
                .ok_or_else(|| {
                    "a font family is a non-empty string or a list of strings".to_owned()
                })
        }
    }
}

/// A curve: an array of keys `{ "x", "y", "easing": [x1, y1, x2, y2] }`, the easing of the
/// last key left out.
fn parse_curve(value: &Json) -> core::result::Result<Value, String> {
    const SHAPE: &str = "a curve is an array of 2 to 8 keys { x, y, easing: [x1, y1, x2, y2] }";
    let items = value
        .as_array()
        .filter(|items| (2..=Curve::MAX_KEYS).contains(&items.len()))
        .ok_or_else(|| SHAPE.to_owned())?;
    let mut keys = [CurveKey::new(0.0, 0.0, CubicBezier::LINEAR); Curve::MAX_KEYS];
    for (key, item) in keys.iter_mut().zip(items) {
        let coordinate = |name: &str| {
            item.get(name)
                .ok_or_else(|| SHAPE.to_owned())
                .and_then(number)
                .map(narrow)
        };
        let easing = match item.get("easing") {
            Some(easing) => {
                let points = easing
                    .as_array()
                    .filter(|points| points.len() == 4)
                    .ok_or_else(|| SHAPE.to_owned())?;
                let mut values = [0.0_f32; 4];
                for (value, point) in values.iter_mut().zip(points) {
                    *value = narrow(number(point)?);
                }
                CubicBezier::new(values[0], values[1], values[2], values[3])
            }
            None => CubicBezier::LINEAR,
        };
        *key = CurveKey::new(coordinate("x")?, coordinate("y")?, easing);
    }
    let curve = Curve::new(&keys[..items.len()]);
    if curve.is_valid() {
        Ok(Value::Curve(curve))
    } else {
        Err(crate::value::CURVE_RULE.to_owned())
    }
}

fn number(value: &Json) -> core::result::Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| format!("expected a number, found {}", value.type_name()))
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "token values are stored as f32; JSON numbers are rounded once here"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

fn amount_and_unit<'a>(
    value: &'a Json,
    what: &str,
) -> core::result::Result<(f64, &'a str), String> {
    let amount = value.get("value").and_then(Json::as_f64);
    let unit = value.get("unit").and_then(Json::as_str);
    match (amount, unit) {
        (Some(amount), Some(unit)) => Ok((amount, unit)),
        _ => Err(format!(
            "a {what} is an object with a number \"value\" and a string \"unit\""
        )),
    }
}

fn font_weight_keyword(keyword: &str) -> Option<u16> {
    Some(match keyword {
        "thin" | "hairline" => 100,
        "extra-light" | "ultra-light" => 200,
        "light" => 300,
        "normal" | "regular" | "book" => 400,
        "medium" => 500,
        "semi-bold" | "demi-bold" => 600,
        "bold" => 700,
        "extra-bold" | "ultra-bold" => 800,
        "black" | "heavy" => 900,
        "extra-black" | "ultra-black" => 950,
        _ => return None,
    })
}

fn parse_color(value: &Json) -> core::result::Result<Color, String> {
    if let Some(text) = value.as_str() {
        return Color::parse_hex(text).map_err(|error| format!("invalid color {text}: {error}"));
    }
    if value.as_object().is_none() {
        return Err(format!(
            "a color is an object with colorSpace and components, found {}",
            value.type_name()
        ));
    }
    let space = value
        .get("colorSpace")
        .and_then(Json::as_str)
        .ok_or_else(|| "a color needs a colorSpace".to_owned())?;
    let components = value
        .get("components")
        .and_then(Json::as_array)
        .filter(|items| items.len() == 3)
        .ok_or_else(|| "a color needs three components".to_owned())?;
    let mut c = [0.0_f64; 3];
    for (slot, item) in c.iter_mut().zip(components) {
        *slot = item
            .as_f64()
            .ok_or_else(|| "color components are numbers".to_owned())?;
    }
    let alpha = match value.get("alpha") {
        None => 1.0,
        Some(alpha) => alpha
            .as_f64()
            .filter(|a| (0.0..=1.0).contains(a))
            .ok_or_else(|| "alpha is a number in 0..=1".to_owned())?,
    };
    let unit = |v: f64| (0.0..=1.0).contains(&v);
    let (c0, c1, c2, a) = (narrow(c[0]), narrow(c[1]), narrow(c[2]), narrow(alpha));
    match space {
        "srgb" if c.iter().all(|&v| unit(v)) => Ok(Color::new(c0, c1, c2, a)),
        "srgb-linear" if c.iter().all(|&v| unit(v)) => Ok(LinearRgba::new(c0, c1, c2, a).to_srgb()),
        "srgb" | "srgb-linear" => Err(format!("{space} components lie in 0..=1")),
        "hsl" => {
            // Saturation and lightness are percentages in the Design Tokens format.
            if !((0.0..360.0).contains(&c[0])
                && (0.0..=100.0).contains(&c[1])
                && (0.0..=100.0).contains(&c[2]))
            {
                return Err(
                    "hsl components: hue in 0..360, saturation and lightness in 0..=100".to_owned(),
                );
            }
            Ok(define::hsla(c0, c1 / 100.0, c2 / 100.0, a).to_srgb())
        }
        "oklab" => {
            if !unit(c[0]) {
                return Err("oklab lightness lies in 0..=1".to_owned());
            }
            Ok(Oklab {
                l: c0,
                a: c1,
                b: c2,
                alpha: a,
            }
            .to_color())
        }
        "oklch" => {
            if !unit(c[0]) || c[1] < 0.0 {
                return Err("oklch lightness lies in 0..=1 and chroma is not negative".to_owned());
            }
            Ok(Oklch::new(c0, c1, c2.rem_euclid(360.0))
                .with_alpha(a)
                .to_color())
        }
        other => Err(format!(
            "unsupported color space {other} (srgb, srgb-linear, hsl, oklab or oklch)"
        )),
    }
}

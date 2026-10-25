//! Export to the Design Tokens format.

use std::collections::HashSet;

use koebi_core::{Color, Oklch};

use crate::dtcg::{EXTENSION, FORMAT};
use crate::error::{Error, Result};
use crate::json::{Json, JsonValue, to_string_pretty};
use crate::kind::KindId;
use crate::layers::{Layers, Origin};
use crate::schema::{CompositeInfo, TokenInfo};
use crate::system::DesignSystem;
use crate::theme::ThemeId;
use crate::tier::TierId;
use crate::token::TokenId;
use crate::value::{ColorLiteral, Def, Definition, Literal, Value};

/// Writes the tokens that `theme` changes compared with the base theme: its own definitions,
/// its overrides and the global overrides, the topmost one for each token.
///
/// # Errors
///
/// [`Error::UnknownTheme`] for a removed or foreign theme.
pub fn export_theme(design: &DesignSystem, theme: ThemeId) -> Result<String> {
    let identity = design
        .theme(theme)
        .ok_or_else(|| Error::UnknownTheme(theme.to_string()))?;
    let graph = design.graph();
    let slot = graph
        .slot(theme)
        .ok_or_else(|| Error::UnknownTheme(theme.to_string()))?;
    let definition = |token: TokenId| graph.winning(token, theme, slot, Layers::DEFINITIONS, None);

    let fields = composite_fields(design);
    let mut root = Vec::new();
    for (id, info) in design.schema().tokens() {
        if fields.contains(&id) {
            continue;
        }
        let (definition, origin) = definition(id);
        if origin == Origin::Base {
            continue;
        }
        let mut extension = token_extension(info);
        extension.push(member("layer", string(layer_name(origin))));
        let value = definition_json(design, definition);
        insert(
            &mut root,
            info.tier(),
            info.path(),
            token_json(info, value, extension),
        );
    }
    for composite in design.schema().composites() {
        let field_ids = field_tokens(composite);
        let changed: Vec<_> = field_ids.iter().map(|&id| definition(id)).collect();
        if changed.iter().all(|(_, origin)| *origin == Origin::Base) {
            continue;
        }
        let value = composite_value(composite, |index| definition_json(design, changed[index].0));
        let extension = vec![member("tier", string(composite.tier().name()))];
        insert(
            &mut root,
            composite.tier(),
            composite.path(),
            composite_json(composite, value, extension),
        );
    }

    let base = design.schema().base_theme().key();
    let mut members = vec![
        member(
            "$description",
            string(&format!(
                "Koebi theme \"{}\": the tokens it changes compared with the base theme \"{base}\".",
                identity.name()
            )),
        ),
        member(
            "$extensions",
            object(vec![member(
                EXTENSION,
                object(vec![
                    member("format", number(FORMAT)),
                    member("schema", string(design.schema().name())),
                    member("base", string(base)),
                    member(
                        "theme",
                        object(vec![
                            member("key", string(identity.key())),
                            member("name", string(identity.name())),
                            member("description", string(identity.description())),
                        ]),
                    ),
                ]),
            )]),
        ),
    ];
    members.extend(root);
    Ok(to_string_pretty(&object(members)))
}

/// Writes every token of the schema with its base definition, its type, description, tier and
/// range: a complete reference of the token set.
pub fn export_schema(design: &DesignSystem) -> String {
    let fields = composite_fields(design);
    let mut root = Vec::new();
    for (id, info) in design.schema().tokens() {
        if fields.contains(&id) {
            continue;
        }
        let value = def_json(design, info.definition());
        insert(
            &mut root,
            info.tier(),
            info.path(),
            token_json(info, value, token_extension(info)),
        );
    }
    for composite in design.schema().composites() {
        let field_ids = field_tokens(composite);
        let value = composite_value(composite, |index| {
            design
                .info(field_ids[index])
                .map_or(Json::new(JsonValue::Null), |info| {
                    def_json(design, info.definition())
                })
        });
        let extension = vec![member("tier", string(composite.tier().name()))];
        insert(
            &mut root,
            composite.tier(),
            composite.path(),
            composite_json(composite, value, extension),
        );
    }
    let mut members = vec![
        member(
            "$description",
            string(&format!(
                "Every token of the Koebi schema \"{}\", with its base definition.",
                design.schema().name()
            )),
        ),
        member(
            "$extensions",
            object(vec![member(
                EXTENSION,
                object(vec![
                    member("format", number(FORMAT)),
                    member("schema", string(design.schema().name())),
                    member("base", string(design.schema().base_theme().key())),
                ]),
            )]),
        ),
    ];
    members.extend(root);
    to_string_pretty(&object(members))
}

fn composite_fields(design: &DesignSystem) -> HashSet<TokenId> {
    design
        .schema()
        .composites()
        .flat_map(field_tokens)
        .collect()
}

fn field_tokens(composite: &CompositeInfo) -> Vec<TokenId> {
    let first = composite.first().get();
    (0..composite.kind().fields().len())
        .filter_map(|offset| u16::try_from(offset).ok())
        .map(|offset| TokenId::from_index(first + offset))
        .collect()
}

fn composite_value(composite: &CompositeInfo, field: impl Fn(usize) -> Json) -> Json {
    object(
        composite
            .kind()
            .fields()
            .iter()
            .enumerate()
            .map(|(index, spec)| member(spec.key, field(index)))
            .collect(),
    )
}

const fn layer_name(origin: Origin) -> &'static str {
    match origin {
        Origin::Base => "base",
        Origin::Theme => "theme",
        Origin::ThemeOverride => "theme-override",
        Origin::GlobalOverride => "global-override",
    }
}

/// The Design Tokens type of a kind; integers are numbers there.
pub(crate) const fn dtcg_type(kind: KindId) -> &'static str {
    match kind {
        KindId::Color => "color",
        KindId::Dimension => "dimension",
        KindId::Number | KindId::Integer => "number",
        KindId::Duration => "duration",
        KindId::CubicBezier => "cubicBezier",
        KindId::FontWeight => "fontWeight",
        KindId::FontFamily => "fontFamily",
    }
}

fn token_extension(info: &TokenInfo) -> Vec<(String, Json)> {
    let mut extension = vec![member("tier", string(info.tier().name()))];
    if info.kind() == KindId::Integer {
        extension.push(member("kind", string("integer")));
    }
    if let Some(range) = info.range() {
        extension.push(member(
            "range",
            object(vec![
                member("min", number(range.min())),
                member("max", number(range.max())),
            ]),
        ));
    }
    extension
}

fn token_json(info: &TokenInfo, value: Json, extension: Vec<(String, Json)>) -> Json {
    object(vec![
        member("$type", string(dtcg_type(info.kind()))),
        member("$value", value),
        member("$description", string(info.description())),
        member(
            "$extensions",
            object(vec![member(EXTENSION, object(extension))]),
        ),
    ])
}

fn composite_json(composite: &CompositeInfo, value: Json, extension: Vec<(String, Json)>) -> Json {
    object(vec![
        member("$type", string(composite.kind().name())),
        member("$value", value),
        member("$description", string(composite.description())),
        member(
            "$extensions",
            object(vec![member(EXTENSION, object(extension))]),
        ),
    ])
}

/// Inserts `token` at `tier.path`, creating the groups on the way.
fn insert(root: &mut Vec<(String, Json)>, tier: TierId, path: &str, token: Json) {
    let mut members = root;
    let segments: Vec<&str> = core::iter::once(tier.name())
        .chain(path.split('.'))
        .collect();
    let (last, groups) = segments.split_last().expect("paths are never empty");
    for segment in groups {
        let index = if let Some(index) = members.iter().position(|(name, _)| name == segment) {
            index
        } else {
            members.push(member(segment, object(Vec::new())));
            members.len() - 1
        };
        members = match &mut members[index].1.value {
            JsonValue::Object(children) => children,
            // Paths never continue a token (checked by the schema), so this is a group.
            _ => return,
        };
    }
    members.push(member(last, token));
}

fn alias_json(design: &DesignSystem, target: TokenId) -> Json {
    let path = design
        .info(target)
        .map_or_else(String::new, |info| info.qualified().to_string());
    string(&format!("{{{path}}}"))
}

fn definition_json(design: &DesignSystem, definition: &Definition) -> Json {
    match definition {
        Definition::Alias(target) => alias_json(design, *target),
        Definition::Value(value) => value_json(value),
    }
}

fn def_json(design: &DesignSystem, def: Def) -> Json {
    match def {
        Def::Alias(target) => alias_json(design, target),
        Def::Literal(Literal::Color(ColorLiteral::Oklch(color))) => oklch_json(color),
        Def::Literal(literal) => value_json(&Value::from(literal)),
    }
}

fn value_json(value: &Value) -> Json {
    match value {
        Value::Color(color) => color_json(*color),
        Value::Dimension(px) => object(vec![
            member("value", number32(*px)),
            member("unit", string("px")),
        ]),
        Value::Number(n) => number32(*n),
        Value::Integer(n) => number(f64::from(*n)),
        Value::Duration(duration) => object(vec![
            member("value", number(duration.as_secs_f64() * 1000.0)),
            member("unit", string("ms")),
        ]),
        Value::CubicBezier(curve) => Json::new(JsonValue::Array(
            [curve.x1, curve.y1, curve.x2, curve.y2]
                .into_iter()
                .map(number32)
                .collect(),
        )),
        Value::FontWeight(weight) => number(f64::from(weight.get())),
        Value::FontFamily(name) => string(name),
    }
}

fn color_json(color: Color) -> Json {
    let [r, g, b, _] = color.to_rgba8();
    object(vec![
        member("colorSpace", string("srgb")),
        member(
            "components",
            Json::new(JsonValue::Array(
                [color.r, color.g, color.b]
                    .into_iter()
                    .map(number32)
                    .collect(),
            )),
        ),
        member("alpha", number32(color.a)),
        member("hex", string(&format!("#{r:02x}{g:02x}{b:02x}"))),
    ])
}

fn oklch_json(color: Oklch) -> Json {
    object(vec![
        member("colorSpace", string("oklch")),
        member(
            "components",
            Json::new(JsonValue::Array(
                [color.l, color.c, color.h]
                    .into_iter()
                    .map(number32)
                    .collect(),
            )),
        ),
        member("alpha", number32(color.alpha)),
    ])
}

fn member(name: &str, value: Json) -> (String, Json) {
    (name.to_owned(), value)
}

fn object(members: Vec<(String, Json)>) -> Json {
    Json::new(JsonValue::Object(members))
}

fn string(text: &str) -> Json {
    Json::new(JsonValue::String(text.to_owned()))
}

fn number(value: f64) -> Json {
    Json::new(JsonValue::Number(value))
}

/// An `f32` written with its shortest decimal form (`0.3647059`, not `0.364705890417099`).
fn number32(value: f32) -> Json {
    number(value.to_string().parse().unwrap_or(f64::from(value)))
}

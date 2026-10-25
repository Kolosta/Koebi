//! The saved state and its binary format.
//!
//! ```text
//! offset  size  content
//!      0     8  signature "KOEBIDS\0"
//!      8     2  format version (1)
//!     10     2  reserved (0)
//!     12     4  payload length
//!     16     4  CRC-32 of the payload
//!     20     n  payload
//! ```
//!
//! The payload holds, in order: the schema name, the active theme's key, the color vision
//! setting, the themes created at run time (key, name, description, definitions), the overrides
//! of each theme (by key), the global overrides and the layer masks that differ from
//! [`Layers::ALL`]. Numbers are little-endian; strings are a `u32` byte length followed by
//! UTF-8. Tokens are saved by path and themes by key, never by position, so saved data survives
//! changes to the schema: what no longer exists is skipped on restore and reported.

use std::sync::Arc;

use crate::error::{Error, Result};
use crate::kind::KindId;
use crate::layers::Layers;
use crate::persist::codec::{Reader, Writer, crc32};
use crate::system::{DesignSystem, Scope};
use crate::theme::{NewTheme, ThemeOrigin};
use crate::token::TokenId;
use crate::value::{CubicBezier, Definition, FontWeight, Value};
use crate::vision::ColorVision;
use koebi_core::{Color, ColorMatrix, Deficiency};

const SIGNATURE: &[u8; 8] = b"KOEBIDS\0";
const VERSION: u16 = 1;
const HEADER: usize = 20;

/// What [`restore`] could not apply, one message per skipped item.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RestoreReport {
    /// Items skipped because their token or theme no longer exists, their value is no longer
    /// valid, or they would close an alias cycle.
    pub skipped: Vec<String>,
}

impl RestoreReport {
    /// `true` when everything was restored.
    pub fn is_complete(&self) -> bool {
        self.skipped.is_empty()
    }
}

/// Encodes the state of `design` that is not part of its schema: themes created at run time,
/// overrides, layer masks, color vision and the active theme.
pub fn encode(design: &DesignSystem) -> Vec<u8> {
    frame(&encode_payload(design))
}

/// The header followed by `payload`.
fn frame(payload: &[u8]) -> Vec<u8> {
    let mut writer = Writer::default();
    for &byte in SIGNATURE {
        writer.u8(byte);
    }
    writer.u16(VERSION);
    writer.u16(0);
    writer.len(payload.len());
    writer.u32(crc32(payload));
    let mut bytes = writer.into_bytes();
    bytes.extend_from_slice(payload);
    bytes
}

/// Restores state saved by [`encode`] into `design`, normally freshly created.
///
/// Items that cannot be applied any more are skipped and listed in the report; the rest is
/// applied. Alias overrides are applied once every value is in place, in whatever order makes
/// each step valid.
///
/// # Errors
///
/// [`Error::Corrupt`] for damaged data, [`Error::UnsupportedVersion`] for a newer format and
/// [`Error::ForeignSchema`] for data saved with another schema. Nothing is changed then.
pub fn restore(design: &mut DesignSystem, bytes: &[u8]) -> Result<RestoreReport> {
    let payload = payload(bytes)?;
    let saved = Saved::read(payload)?;
    if saved.schema != design.schema().name() {
        return Err(Error::ForeignSchema(saved.schema.to_owned()));
    }
    let mut report = RestoreReport::default();
    saved.apply(design, &mut report);
    Ok(report)
}

/// Checks the header and checksum and returns the payload.
fn payload(bytes: &[u8]) -> Result<&[u8]> {
    if bytes.len() < HEADER || &bytes[..8] != SIGNATURE {
        return Err(Error::Corrupt("not Koebi design data".to_owned()));
    }
    let mut header = Reader::new(&bytes[8..HEADER]);
    let version = header.u16()?;
    if version > VERSION {
        return Err(Error::UnsupportedVersion(version));
    }
    let _reserved = header.u16()?;
    let len = usize::try_from(header.u32()?).unwrap_or(usize::MAX);
    let checksum = header.u32()?;
    let payload = &bytes[HEADER..];
    if payload.len() != len {
        return Err(Error::Corrupt("truncated data".to_owned()));
    }
    if crc32(payload) != checksum {
        return Err(Error::Corrupt("checksum mismatch".to_owned()));
    }
    Ok(payload)
}

fn encode_payload(design: &DesignSystem) -> Vec<u8> {
    let mut writer = Writer::default();
    writer.str(design.schema().name());
    let active = design
        .theme(design.active_theme())
        .map_or("", |theme| theme.key());
    writer.str(active);
    write_vision(&mut writer, design.color_vision());

    let user: Vec<_> = design
        .themes()
        .filter(|(_, theme)| theme.origin() == ThemeOrigin::User)
        .collect();
    writer.len(user.len());
    for (id, theme) in user {
        writer.str(theme.key());
        writer.str(theme.name());
        writer.str(theme.description());
        let definitions: Vec<_> = design.theme_definitions(id).into_iter().flatten().collect();
        write_definitions(&mut writer, design, definitions);
    }

    let overridden: Vec<_> = design
        .themes()
        .filter_map(|(id, theme)| {
            let definitions: Vec<_> = design.overrides(Scope::Theme(id)).ok()?.collect();
            (!definitions.is_empty()).then_some((theme.key(), definitions))
        })
        .collect();
    writer.len(overridden.len());
    for (key, definitions) in overridden {
        writer.str(key);
        write_definitions(&mut writer, design, definitions);
    }

    let global: Vec<_> = design
        .overrides(Scope::Global)
        .map(Iterator::collect)
        .unwrap_or_default();
    write_definitions(&mut writer, design, global);

    let masks: Vec<_> = design
        .schema()
        .tokens()
        .filter(|(id, _)| design.token_layers(*id) != Layers::ALL)
        .collect();
    writer.len(masks.len());
    for (id, info) in masks {
        writer.str(&info.qualified().to_string());
        writer.u8(design.token_layers(id).bits());
    }
    writer.into_bytes()
}

fn path(design: &DesignSystem, token: TokenId) -> String {
    design
        .info(token)
        .map_or_else(String::new, |info| info.qualified().to_string())
}

fn write_definitions(
    writer: &mut Writer,
    design: &DesignSystem,
    mut definitions: Vec<(TokenId, &Definition)>,
) {
    // Sorted by token, so that the same state always gives the same bytes.
    definitions.sort_by_key(|(token, _)| *token);
    writer.len(definitions.len());
    for (token, definition) in definitions {
        writer.str(&path(design, token));
        match definition {
            Definition::Alias(target) => {
                writer.u8(0);
                writer.str(&path(design, *target));
            }
            Definition::Value(value) => {
                writer.u8(1);
                write_value(writer, value);
            }
        }
    }
}

fn write_value(writer: &mut Writer, value: &Value) {
    writer.u8(value.kind() as u8);
    match value {
        Value::Color(c) => [c.r, c.g, c.b, c.a].into_iter().for_each(|v| writer.f32(v)),
        Value::Dimension(v) | Value::Number(v) => writer.f32(*v),
        Value::Integer(v) => writer.i32(*v),
        Value::Duration(d) => {
            writer.u64(d.as_secs());
            writer.u32(d.subsec_nanos());
        }
        Value::CubicBezier(b) => [b.x1, b.y1, b.x2, b.y2]
            .into_iter()
            .for_each(|v| writer.f32(v)),
        Value::FontWeight(w) => writer.u16(w.get()),
        Value::FontFamily(name) => writer.str(name),
    }
}

fn write_vision(writer: &mut Writer, vision: ColorVision) {
    match vision {
        ColorVision::Normal => writer.u8(0),
        ColorVision::Correct {
            deficiency,
            strength,
        } => {
            writer.u8(1);
            writer.u8(deficiency_code(deficiency));
            writer.f32(strength);
        }
        ColorVision::Simulate {
            deficiency,
            severity,
        } => {
            writer.u8(2);
            writer.u8(deficiency_code(deficiency));
            writer.f32(severity);
        }
        ColorVision::SimulateAchromatopsia { severity } => {
            writer.u8(3);
            writer.f32(severity);
        }
        ColorVision::Custom(matrix) => {
            writer.u8(4);
            matrix
                .rows()
                .into_iter()
                .flatten()
                .for_each(|v| writer.f32(v));
        }
    }
}

const fn deficiency_code(deficiency: Deficiency) -> u8 {
    match deficiency {
        Deficiency::Protan => 0,
        Deficiency::Deutan => 1,
        Deficiency::Tritan => 2,
    }
}

/// A definition as saved: paths instead of handles.
#[derive(Debug)]
enum SavedDefinition<'a> {
    Alias(&'a str),
    Value(Value),
}

#[derive(Debug)]
struct SavedEntry<'a> {
    token: &'a str,
    definition: SavedDefinition<'a>,
}

#[derive(Debug)]
struct SavedTheme<'a> {
    key: &'a str,
    name: &'a str,
    description: &'a str,
    definitions: Vec<SavedEntry<'a>>,
}

/// The saved state, parsed in full before anything is applied.
#[derive(Debug)]
struct Saved<'a> {
    schema: &'a str,
    active: &'a str,
    vision: ColorVision,
    themes: Vec<SavedTheme<'a>>,
    overrides: Vec<(&'a str, Vec<SavedEntry<'a>>)>,
    global: Vec<SavedEntry<'a>>,
    masks: Vec<(&'a str, u8)>,
}

impl<'a> Saved<'a> {
    fn read(payload: &'a [u8]) -> Result<Self> {
        let mut reader = Reader::new(payload);
        let schema = reader.str()?;
        let active = reader.str()?;
        let vision = read_vision(&mut reader)?;
        let mut themes = Vec::new();
        for _ in 0..reader.count()? {
            themes.push(SavedTheme {
                key: reader.str()?,
                name: reader.str()?,
                description: reader.str()?,
                definitions: read_entries(&mut reader)?,
            });
        }
        let mut overrides = Vec::new();
        for _ in 0..reader.count()? {
            overrides.push((reader.str()?, read_entries(&mut reader)?));
        }
        let global = read_entries(&mut reader)?;
        let mut masks = Vec::new();
        for _ in 0..reader.count()? {
            masks.push((reader.str()?, reader.u8()?));
        }
        if !reader.is_empty() {
            return Err(Error::Corrupt("unexpected data after the end".to_owned()));
        }
        Ok(Self {
            schema,
            active,
            vision,
            themes,
            overrides,
            global,
            masks,
        })
    }

    fn apply(&self, design: &mut DesignSystem, report: &mut RestoreReport) {
        for theme in &self.themes {
            restore_theme(design, theme, report);
        }
        for &(path, bits) in &self.masks {
            let mask = Layers::from_bits(bits);
            match (design.find(path), mask) {
                (Some(token), Some(layers)) => {
                    if let Err(error) = design.set_token_layers(token, layers) {
                        report
                            .skipped
                            .push(format!("layer mask of {path}: {error}"));
                    }
                }
                (None, _) => report
                    .skipped
                    .push(format!("layer mask of {path}: unknown token")),
                (_, None) => report
                    .skipped
                    .push(format!("layer mask of {path}: invalid layers")),
            }
        }

        let mut pending = Vec::new();
        for (key, entries) in &self.overrides {
            let Some(theme) = design.find_theme(key) else {
                report
                    .skipped
                    .push(format!("overrides of theme {key}: unknown theme"));
                continue;
            };
            collect(
                design,
                Scope::Theme(theme),
                entries,
                &format!("theme {key}"),
                &mut pending,
                report,
            );
        }
        collect(
            design,
            Scope::Global,
            &self.global,
            "global",
            &mut pending,
            report,
        );
        apply_overrides(design, pending, report);

        if let Err(error) = design.set_color_vision(self.vision) {
            report.skipped.push(format!("color vision: {error}"));
        }
        match design.find_theme(self.active) {
            Some(theme) => {
                if let Err(error) = design.set_active_theme(theme) {
                    report.skipped.push(format!("active theme: {error}"));
                }
            }
            None => report
                .skipped
                .push(format!("active theme {}: unknown theme", self.active)),
        }
    }
}

/// An override ready to apply.
struct Pending {
    scope: Scope,
    token: TokenId,
    definition: Definition,
    label: String,
}

/// Resolves saved entries to handles, checking kinds, ranges and tier rules.
fn collect(
    design: &DesignSystem,
    scope: Scope,
    entries: &[SavedEntry<'_>],
    owner: &str,
    pending: &mut Vec<Pending>,
    report: &mut RestoreReport,
) {
    for entry in entries {
        let label = format!("{owner} override of {}", entry.token);
        match resolve(design, entry) {
            Ok((token, definition)) => pending.push(Pending {
                scope,
                token,
                definition,
                label,
            }),
            Err(reason) => report.skipped.push(format!("{label}: {reason}")),
        }
    }
}

/// Applies values first, then aliases in passes until no alias can be applied any more.
fn apply_overrides(design: &mut DesignSystem, pending: Vec<Pending>, report: &mut RestoreReport) {
    let (values, mut aliases): (Vec<_>, Vec<_>) = pending
        .into_iter()
        .partition(|p| matches!(p.definition, Definition::Value(_)));
    for p in values {
        if let Err(error) = design.set_override(p.scope, p.token, p.definition) {
            report.skipped.push(format!("{}: {error}", p.label));
        }
    }
    let mut errors: Vec<String> = Vec::new();
    loop {
        let before = aliases.len();
        errors.clear();
        aliases.retain(
            |p| match design.set_override(p.scope, p.token, p.definition.clone()) {
                Ok(()) => false,
                Err(error) => {
                    errors.push(format!("{}: {error}", p.label));
                    true
                }
            },
        );
        if aliases.is_empty() || aliases.len() == before {
            break;
        }
    }
    report.skipped.extend(errors);
}

fn resolve(
    design: &DesignSystem,
    entry: &SavedEntry<'_>,
) -> core::result::Result<(TokenId, Definition), String> {
    let token = design
        .find(entry.token)
        .ok_or_else(|| "unknown token".to_owned())?;
    let definition = match &entry.definition {
        SavedDefinition::Alias(target) => Definition::Alias(
            design
                .find(target)
                .ok_or_else(|| format!("unknown alias target {target}"))?,
        ),
        SavedDefinition::Value(value) => Definition::Value(value.clone()),
    };
    design
        .check_definition(token, definition)
        .map(|definition| (token, definition))
        .map_err(|error| error.to_string())
}

fn restore_theme(design: &mut DesignSystem, theme: &SavedTheme<'_>, report: &mut RestoreReport) {
    let mut definitions = Vec::new();
    for entry in &theme.definitions {
        match resolve(design, entry) {
            Ok(definition) => definitions.push(definition),
            Err(reason) => report
                .skipped
                .push(format!("theme {}: {}: {reason}", theme.key, entry.token)),
        }
    }
    // A definition that closes a cycle (after a schema change) is dropped, then retried.
    loop {
        let new = NewTheme {
            key: theme.key.to_owned(),
            name: theme.name.to_owned(),
            description: theme.description.to_owned(),
            definitions: definitions.clone(),
        };
        match design.create_theme(new) {
            Ok(_) => return,
            Err(Error::Cycle { token }) => {
                let before = definitions.len();
                definitions.retain(|(id, _)| path(design, *id) != token);
                report.skipped.push(format!(
                    "theme {}: {token}: would create an alias cycle",
                    theme.key
                ));
                if definitions.len() == before {
                    return;
                }
            }
            Err(error) => {
                report.skipped.push(format!("theme {}: {error}", theme.key));
                return;
            }
        }
    }
}

fn read_entries<'a>(reader: &mut Reader<'a>) -> Result<Vec<SavedEntry<'a>>> {
    let count = reader.count()?;
    let mut entries = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let token = reader.str()?;
        let definition = match reader.u8()? {
            0 => SavedDefinition::Alias(reader.str()?),
            1 => SavedDefinition::Value(read_value(reader)?),
            tag => return Err(Error::Corrupt(format!("unknown definition tag {tag}"))),
        };
        entries.push(SavedEntry { token, definition });
    }
    Ok(entries)
}

fn read_value(reader: &mut Reader<'_>) -> Result<Value> {
    let code = reader.u8()?;
    let kind = KindId::ALL
        .into_iter()
        .find(|kind| *kind as u8 == code)
        .ok_or_else(|| Error::Corrupt(format!("unknown kind {code}")))?;
    Ok(match kind {
        KindId::Color => Value::Color(Color::new(
            reader.f32()?,
            reader.f32()?,
            reader.f32()?,
            reader.f32()?,
        )),
        KindId::Dimension => Value::Dimension(reader.f32()?),
        KindId::Number => Value::Number(reader.f32()?),
        KindId::Integer => Value::Integer(reader.i32()?),
        KindId::Duration => {
            let seconds = reader.u64()?;
            let nanos = reader.u32()?;
            if nanos >= 1_000_000_000 {
                return Err(Error::Corrupt("invalid duration".to_owned()));
            }
            Value::Duration(core::time::Duration::new(seconds, nanos))
        }
        KindId::CubicBezier => Value::CubicBezier(CubicBezier::new(
            reader.f32()?,
            reader.f32()?,
            reader.f32()?,
            reader.f32()?,
        )),
        KindId::FontWeight => Value::FontWeight(
            FontWeight::new(reader.u16()?)
                .ok_or_else(|| Error::Corrupt("invalid font weight".to_owned()))?,
        ),
        KindId::FontFamily => Value::FontFamily(Arc::from(reader.str()?)),
    })
}

fn read_vision(reader: &mut Reader<'_>) -> Result<ColorVision> {
    let deficiency = |code: u8| match code {
        0 => Ok(Deficiency::Protan),
        1 => Ok(Deficiency::Deutan),
        2 => Ok(Deficiency::Tritan),
        _ => Err(Error::Corrupt(format!("unknown deficiency {code}"))),
    };
    Ok(match reader.u8()? {
        0 => ColorVision::Normal,
        1 => ColorVision::Correct {
            deficiency: deficiency(reader.u8()?)?,
            strength: reader.f32()?,
        },
        2 => ColorVision::Simulate {
            deficiency: deficiency(reader.u8()?)?,
            severity: reader.f32()?,
        },
        3 => ColorVision::SimulateAchromatopsia {
            severity: reader.f32()?,
        },
        4 => {
            let mut rows = [[0.0; 3]; 3];
            for value in rows.iter_mut().flatten() {
                *value = reader.f32()?;
            }
            ColorVision::Custom(ColorMatrix::new(rows))
        }
        tag => return Err(Error::Corrupt(format!("unknown color vision {tag}"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Schema, ThemeInfo};

    #[allow(
        dead_code,
        unreachable_pub,
        reason = "a small schema for the tests; not every token is read"
    )]
    mod tokens {
        use crate::define::*;
        crate::tokens! {
            group PRIMITIVE: Primitive, start = 0;
            /// A color.
            COLOR_A = "color.a": Color = hex("#112233");
            /// A radius.
            RADIUS_A = "radius.a": Dimension = px(2.0), range(0.0, 8.0);
        }
        crate::tokens! {
            group SEMANTIC: Semantic, start = PRIMITIVE::END;
            /// A background.
            BACKGROUND = "background": Color -> COLOR_A;
        }
    }

    static SCHEMA: Schema = Schema::new(
        "unit",
        ThemeInfo::new("base", "Base", "Base theme.", &[]),
        &[tokens::PRIMITIVE::GROUP, tokens::SEMANTIC::GROUP],
        &[],
    );

    fn entry(writer: &mut Writer, token: &str, alias: Option<&str>) {
        writer.str(token);
        if let Some(target) = alias {
            writer.u8(0);
            writer.str(target);
        } else {
            writer.u8(1);
            write_value(writer, &Value::Dimension(4.0));
        }
    }

    /// A payload naming tokens, themes and masks that do not exist.
    fn payload_with_unknowns(schema: &str) -> Vec<u8> {
        let mut writer = Writer::default();
        writer.str(schema);
        writer.str("gone");
        write_vision(&mut writer, ColorVision::Normal);
        writer.len(0);
        writer.len(1);
        writer.str("removed-theme");
        writer.len(0);
        writer.len(3);
        entry(&mut writer, "primitive.radius.a", None);
        entry(&mut writer, "primitive.missing", None);
        entry(
            &mut writer,
            "semantic.background",
            Some("primitive.color.missing"),
        );
        writer.len(1);
        writer.str("semantic.missing");
        writer.u8(Layers::NONE.bits());
        writer.into_bytes()
    }

    #[test]
    fn unknown_tokens_and_themes_are_skipped_and_reported() {
        let mut design = DesignSystem::new(&SCHEMA).unwrap();
        let report = restore(&mut design, &frame(&payload_with_unknowns("unit"))).unwrap();
        assert_eq!(
            design.get(tokens::RADIUS_A),
            4.0,
            "valid entries are applied"
        );
        let skipped = report.skipped.join(
            "
",
        );
        for expected in [
            "theme removed-theme: unknown theme",
            "primitive.missing: unknown token",
            "unknown alias target primitive.color.missing",
            "layer mask of semantic.missing: unknown token",
            "active theme gone: unknown theme",
        ] {
            assert!(
                skipped.contains(expected),
                "missing {expected:?} in:
{skipped}"
            );
        }
    }

    #[test]
    fn data_of_another_schema_is_refused() {
        let mut design = DesignSystem::new(&SCHEMA).unwrap();
        let result = restore(&mut design, &frame(&payload_with_unknowns("other")));
        assert_eq!(result, Err(Error::ForeignSchema("other".to_owned())));
        assert_eq!(design.revision(), 0);
    }
}

//! Schema validation: compile-time checks (`const fn`) and the complete run-time check.
//!
//! Each rule is a `const fn` returning `Option<&'static str>` (the problem, if any) so that
//! [`Schema::assert_valid`] and [`Schema::validate`] share it: the first panics with a message
//! naming the token (a compile error in a `const` item), the second collects every problem.

use std::collections::HashSet;

use crate::kind::KindId;
use crate::schema::{CompositeInfo, Group, Schema, ThemeInfo, TokenInfo};
use crate::tier::TierId;
use crate::value::{Def, Literal};

/// A message assembled at compile time from static parts (const panics take one `&str`).
struct Message {
    bytes: [u8; 240],
    len: usize,
}

impl Message {
    const fn new() -> Self {
        Self {
            bytes: [0; 240],
            len: 0,
        }
    }

    const fn push(mut self, text: &str) -> Self {
        let text = text.as_bytes();
        let mut i = 0;
        while i < text.len() && self.len < self.bytes.len() {
            self.bytes[self.len] = text[i];
            self.len += 1;
            i += 1;
        }
        self
    }

    const fn as_str(&self) -> &str {
        match core::str::from_utf8(self.bytes.split_at(self.len).0) {
            Ok(text) => text,
            Err(_) => "invalid token table",
        }
    }
}

#[allow(
    clippy::panic,
    reason = "schemas are constants: the panic is a compile-time error naming the token"
)]
const fn fail(problem: &str, tier: TierId, path: &str) -> ! {
    let message = Message::new()
        .push(problem)
        .push(": ")
        .push(tier.name())
        .push(".")
        .push(path);
    panic!("{}", message.as_str())
}

const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Paths are lowercase segments of `a-z` and `0-9`, separated by `.`; `-` joins words inside a
/// segment (`kbd-focus`).
pub(crate) const fn path_problem(path: &str) -> Option<&'static str> {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return Some("empty token path");
    }
    let mut i = 0;
    let mut segment_start = true;
    while i < bytes.len() {
        let b = bytes[i];
        let at_end = i + 1 == bytes.len() || bytes[i + 1] == b'.';
        match b {
            b'a'..=b'z' | b'0'..=b'9' => {}
            b'.' => {
                if segment_start {
                    return Some("empty segment in token path");
                }
            }
            b'-' => {
                if segment_start || at_end || bytes[i + 1] == b'-' {
                    return Some("misplaced `-` in token path");
                }
            }
            _ => return Some("token paths only use `a-z`, `0-9`, `.` and `-`"),
        }
        segment_start = b == b'.';
        i += 1;
    }
    if segment_start {
        return Some("empty segment in token path");
    }
    None
}

/// The Rust constant of a token is its path in upper case, `.` and `-` becoming `_`.
pub(crate) const fn ident_problem(path: &str, ident: &str) -> Option<&'static str> {
    let (p, n) = (path.as_bytes(), ident.as_bytes());
    if p.len() != n.len() {
        return Some("constant name does not match the path (expected the path in upper case)");
    }
    let mut i = 0;
    while i < p.len() {
        let expected = match p[i] {
            b'.' | b'-' => b'_',
            b'a'..=b'z' => p[i] - b'a' + b'A',
            other => other,
        };
        if n[i] != expected {
            return Some("constant name does not match the path (expected the path in upper case)");
        }
        i += 1;
    }
    None
}

/// A literal must match the token's kind, be finite and lie in the token's range.
pub(crate) const fn literal_problem(info: &TokenInfo, literal: &Literal) -> Option<&'static str> {
    if literal.kind() as u8 != info.kind() as u8 {
        return Some("literal of the wrong kind");
    }
    match *literal {
        Literal::Color(color) => {
            if !color.is_valid() {
                return Some("color channel out of range");
            }
        }
        Literal::Dimension(v) | Literal::Number(v) => {
            if !v.is_finite() {
                return Some("number is not finite");
            }
        }
        Literal::CubicBezier(curve) => {
            if !curve.is_valid() {
                return Some("cubic Bézier x coordinates must lie in 0..=1");
            }
        }
        Literal::FontFamily(name) => {
            if name.is_empty() {
                return Some("empty font family name");
            }
        }
        Literal::Integer(_) | Literal::Duration(_) | Literal::FontWeight(_) => {}
    }
    if let (Some(range), Some(value)) = (info.range(), literal.as_number())
        && !range.contains(value)
    {
        return Some("value outside the token's range");
    }
    None
}

/// Checks one token on its own (no lookup in other groups).
pub(crate) const fn token_problem(info: &TokenInfo) -> Option<&'static str> {
    if let Some(problem) = path_problem(info.path()) {
        return Some(problem);
    }
    if !info.ident().is_empty()
        && let Some(problem) = ident_problem(info.path(), info.ident())
    {
        return Some(problem);
    }
    if let Some(range) = info.range() {
        if !range.is_valid() {
            return Some("invalid range");
        }
        if !info.kind().is_numeric() {
            return Some("only numeric kinds have a range");
        }
    }
    match info.definition() {
        Def::Literal(literal) => {
            if info.tier() as u8 == TierId::Semantic as u8 {
                return Some(
                    "semantic tokens only alias; primitive and component tokens hold values",
                );
            }
            literal_problem(info, &literal)
        }
        Def::Alias(_) => {
            if info.tier() as u8 == TierId::Primitive as u8 {
                return Some("primitive tokens hold values and cannot alias");
            }
            None
        }
    }
}

/// An alias must point at an existing token of the same kind, in a tier the rules allow.
pub(crate) const fn alias_problem(
    from_tier: TierId,
    from_kind: KindId,
    target: Option<&TokenInfo>,
) -> Option<&'static str> {
    match target {
        None => Some("alias to a token outside the schema"),
        Some(target) => {
            if target.kind() as u8 != from_kind as u8 {
                Some("alias to a token of another kind")
            } else if !from_tier.may_alias(target.tier()) {
                Some("alias breaks the tier rules")
            } else {
                None
            }
        }
    }
}

/// Checks the tokens of a group. Called by the token table macros in a `const` item.
///
/// # Panics
///
/// With a message naming the first broken rule and token.
#[doc(hidden)]
pub const fn assert_group(group: &Group) {
    let tokens = group.tokens();
    let mut i = 0;
    while i < tokens.len() {
        let info = &tokens[i];
        if let Some(problem) = token_problem(info) {
            fail(problem, info.tier(), info.path());
        }
        i += 1;
    }
}

/// The token at `index`, by binary search over the groups (sorted by position).
const fn info_at(schema: &Schema, index: usize) -> Option<&'static TokenInfo> {
    let groups = schema.groups();
    let (mut low, mut high) = (0, groups.len());
    while low < high {
        let middle = low + (high - low) / 2;
        let group = &groups[middle];
        let start = group.start() as usize;
        if index < start {
            high = middle;
        } else if index >= group.end() {
            low = middle + 1;
        } else {
            return Some(&group.tokens()[index - start]);
        }
    }
    None
}

/// Index of the alias target of `index` in `theme` (or in the base theme), if it is an alias.
const fn alias_in(schema: &Schema, theme: Option<&ThemeInfo>, index: usize) -> Option<usize> {
    if let Some(theme) = theme {
        let entries = theme.entries();
        let mut e = 0;
        while e < entries.len() {
            if entries[e].token().index() == index {
                return match entries[e].definition() {
                    Def::Alias(target) => Some(target.index()),
                    Def::Literal(_) => None,
                };
            }
            e += 1;
        }
    }
    match info_at(schema, index) {
        Some(info) => match info.definition() {
            Def::Alias(target) => Some(target.index()),
            Def::Literal(_) => None,
        },
        None => None,
    }
}

/// `true` when following aliases from `index` never ends.
///
/// The base graph is checked from every token. A theme can only close a cycle through a token
/// it redefines, so themes are checked from their entries only.
const fn has_cycle(schema: &Schema, theme: Option<&ThemeInfo>, index: usize) -> bool {
    let len = schema.len();
    let mut current = index;
    let mut steps = 0;
    while steps <= len {
        match alias_in(schema, theme, current) {
            Some(next) => current = next,
            None => return false,
        }
        steps += 1;
    }
    true
}

pub(crate) const fn theme_key_problem(key: &str) -> Option<&'static str> {
    let bytes = key.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return Some("theme keys have 1 to 64 characters");
    }
    let mut i = 0;
    while i < bytes.len() {
        let valid = matches!(bytes[i], b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.');
        if !valid {
            return Some("theme keys only use `a-z`, `0-9`, `-` and `.`");
        }
        i += 1;
    }
    None
}

/// Checks a schema at compile time. See [`Schema::assert_valid`].
pub(crate) const fn assert_schema(schema: &Schema) {
    let groups = schema.groups();
    let mut expected = 0_usize;
    let mut g = 0;
    while g < groups.len() {
        let group = &groups[g];
        if group.start() as usize != expected {
            fail(
                "group does not start where the previous one ends",
                TierId::Primitive,
                group.name(),
            );
        }
        expected = group.end();
        g += 1;
    }
    if expected >= u16::MAX as usize {
        fail("too many tokens", TierId::Primitive, schema.name());
    }

    let len = schema.len();
    let mut index = 0;
    while index < len {
        // Each token was checked on its own by its group (`assert_group`).
        let Some(info) = info_at(schema, index) else {
            fail("missing token", TierId::Primitive, schema.name())
        };
        if let Def::Alias(target) = info.definition() {
            let problem = alias_problem(info.tier(), info.kind(), info_at(schema, target.index()));
            if let Some(problem) = problem {
                fail(problem, info.tier(), info.path());
            }
        }
        if has_cycle(schema, None, index) {
            fail("alias cycle", info.tier(), info.path());
        }
        index += 1;
    }

    if !schema.base_theme().entries().is_empty() {
        fail(
            "the base theme cannot have entries",
            TierId::Primitive,
            schema.base_theme().key(),
        );
    }
    let themes = schema.themes();
    let mut t = 0;
    while t < themes.len() {
        assert_theme(schema, &themes[t]);
        let mut other = 0;
        while other < t {
            if bytes_eq(themes[other].key().as_bytes(), themes[t].key().as_bytes()) {
                fail("duplicate theme key", TierId::Primitive, themes[t].key());
            }
            other += 1;
        }
        if bytes_eq(
            schema.base_theme().key().as_bytes(),
            themes[t].key().as_bytes(),
        ) {
            fail("duplicate theme key", TierId::Primitive, themes[t].key());
        }
        t += 1;
    }
    if let Some(problem) = theme_key_problem(schema.base_theme().key()) {
        fail(problem, TierId::Primitive, schema.base_theme().key());
    }

    let mut g = 0;
    while g < groups.len() {
        let composites = groups[g].composites();
        let mut c = 0;
        while c < composites.len() {
            if let Some(problem) = composite_problem(schema, &composites[c]) {
                fail(problem, composites[c].tier(), composites[c].path());
            }
            c += 1;
        }
        g += 1;
    }
}

const fn assert_theme(schema: &Schema, theme: &ThemeInfo) {
    if let Some(problem) = theme_key_problem(theme.key()) {
        fail(problem, TierId::Primitive, theme.key());
    }
    let entries = theme.entries();
    let mut e = 0;
    while e < entries.len() {
        let entry = &entries[e];
        let Some(info) = info_at(schema, entry.token().index()) else {
            fail(
                "theme entry for a token outside the schema",
                TierId::Primitive,
                theme.key(),
            )
        };
        let problem = match entry.definition() {
            Def::Alias(target) => {
                alias_problem(info.tier(), info.kind(), info_at(schema, target.index()))
            }
            Def::Literal(literal) => literal_problem(info, &literal),
        };
        if let Some(problem) = problem {
            fail(problem, info.tier(), info.path());
        }
        let mut other = 0;
        while other < e {
            if entries[other].token().index() == entry.token().index() {
                fail("token defined twice by a theme", info.tier(), info.path());
            }
            other += 1;
        }
        e += 1;
    }
    let mut e = 0;
    while e < entries.len() {
        let index = entries[e].token().index();
        if has_cycle(schema, Some(theme), index) {
            match info_at(schema, index) {
                Some(info) => fail("alias cycle in a theme", info.tier(), info.path()),
                None => fail("alias cycle in a theme", TierId::Primitive, theme.key()),
            }
        }
        e += 1;
    }
}

/// A composite's fields must be its next tokens, named `<composite>.<segment>`, of the right
/// kinds, in the composite's tier.
pub(crate) const fn composite_problem(
    schema: &Schema,
    composite: &CompositeInfo,
) -> Option<&'static str> {
    if let Some(problem) = path_problem(composite.path()) {
        return Some(problem);
    }
    let fields = composite.kind().fields();
    let first = composite.first().index();
    let mut f = 0;
    while f < fields.len() {
        let Some(info) = info_at(schema, first + f) else {
            return Some("composite field outside the schema");
        };
        if info.kind() as u8 != fields[f].kind as u8 {
            return Some("composite field of the wrong kind");
        }
        if info.tier() as u8 != composite.tier() as u8 {
            return Some("composite field in another tier");
        }
        let (path, prefix, segment) = (
            info.path().as_bytes(),
            composite.path().as_bytes(),
            fields[f].segment.as_bytes(),
        );
        let named = path.len() == prefix.len() + 1 + segment.len()
            && bytes_eq(path.split_at(prefix.len()).0, prefix)
            && path[prefix.len()] == b'.'
            && bytes_eq(path.split_at(prefix.len() + 1).1, segment);
        if !named {
            return Some("composite field path is not `<composite>.<field>`");
        }
        f += 1;
    }
    None
}

/// Every rule, collected instead of panicking, plus path uniqueness and nesting.
pub(crate) fn validate_schema(schema: &Schema) -> Vec<String> {
    let mut problems = Vec::new();
    let mut expected = 0;
    for group in schema.groups() {
        if usize::from(group.start()) != expected {
            problems.push(format!(
                "group {} does not start where the previous one ends",
                group.name()
            ));
        }
        expected = group.end();
    }
    if expected >= usize::from(u16::MAX) {
        problems.push(format!("too many tokens ({expected})"));
        return problems;
    }
    for (id, info) in schema.tokens() {
        let mut report = |problem: &str| problems.push(format!("{}: {problem}", info.qualified()));
        if let Some(problem) = token_problem(info) {
            report(problem);
        }
        if let Def::Alias(target) = info.definition()
            && let Some(problem) = alias_problem(info.tier(), info.kind(), schema.token(target))
        {
            report(problem);
        }
        if has_cycle(schema, None, id.index()) {
            report("alias cycle");
        }
    }
    let base = schema.base_theme();
    if !base.entries().is_empty() {
        problems.push(format!(
            "base theme {}: the base theme cannot have entries",
            base.key()
        ));
    }
    let mut keys = vec![base.key()];
    for theme in schema.themes() {
        if keys.contains(&theme.key()) {
            problems.push(format!("theme {}: duplicate theme key", theme.key()));
        }
        keys.push(theme.key());
        validate_theme(schema, theme, &mut problems);
    }
    for key in keys {
        if let Some(problem) = theme_key_problem(key) {
            problems.push(format!("theme {key}: {problem}"));
        }
    }
    for composite in schema.composites() {
        if let Some(problem) = composite_problem(schema, composite) {
            problems.push(format!("{}: {problem}", composite.qualified()));
        }
    }
    validate_paths(schema, &mut problems);
    problems
}

fn validate_theme(schema: &Schema, theme: &ThemeInfo, problems: &mut Vec<String>) {
    let mut seen = Vec::new();
    for entry in theme.entries() {
        let Some(info) = schema.token(entry.token()) else {
            problems.push(format!(
                "theme {}: entry for a token outside the schema",
                theme.key()
            ));
            continue;
        };
        let problem = match entry.definition() {
            Def::Alias(target) => alias_problem(info.tier(), info.kind(), schema.token(target)),
            Def::Literal(literal) => literal_problem(info, &literal),
        };
        if let Some(problem) = problem {
            problems.push(format!(
                "theme {}: {}: {problem}",
                theme.key(),
                info.qualified()
            ));
        }
        if seen.contains(&entry.token()) {
            problems.push(format!(
                "theme {}: {}: token defined twice by a theme",
                theme.key(),
                info.qualified()
            ));
        }
        seen.push(entry.token());
    }
    for entry in theme.entries() {
        if has_cycle(schema, Some(theme), entry.token().index()) {
            let name = schema.token(entry.token()).map_or_else(
                || entry.token().to_string(),
                |info| info.qualified().to_string(),
            );
            problems.push(format!("theme {}: {name}: alias cycle", theme.key()));
        }
    }
}

/// Paths are unique, and a token is never a group: no token path continues another token's
/// path or a composite's path.
fn validate_paths(schema: &Schema, problems: &mut Vec<String>) {
    let mut paths = HashSet::new();
    for (_, info) in schema.tokens() {
        if !paths.insert((info.tier(), info.path())) {
            problems.push(format!("{}: duplicate token path", info.qualified()));
        }
    }
    for (_, info) in schema.tokens() {
        let path = info.path();
        for (end, _) in path.match_indices('.') {
            if paths.contains(&(info.tier(), &path[..end])) {
                problems.push(format!(
                    "{}: path inside the token {}.{}",
                    info.qualified(),
                    info.tier(),
                    &path[..end]
                ));
            }
        }
    }
    for composite in schema.composites() {
        if paths.contains(&(composite.tier(), composite.path())) {
            problems.push(format!(
                "{}: a token has the path of a composite",
                composite.qualified()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_follow_the_naming_rules() {
        assert_eq!(path_problem("background.accent.kbd-focus"), None);
        assert_eq!(path_problem("size.2xl"), None);
        for bad in [
            "", ".a", "a.", "a..b", "-a", "a-", "a--b", "a.-b", "A.b", "a_b", "a b",
        ] {
            assert!(path_problem(bad).is_some(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn constants_mirror_paths() {
        assert_eq!(
            ident_problem("background.accent.kbd-focus", "BACKGROUND_ACCENT_KBD_FOCUS"),
            None
        );
        assert_eq!(ident_problem("size.2xl", "SIZE_2XL"), None);
        assert!(ident_problem("background.base", "BACKGROUND_BASE_DEFAULT").is_some());
        assert!(ident_problem("background.base", "BACKGROUND_BASIC").is_some());
    }

    #[test]
    fn theme_keys_are_restricted() {
        assert_eq!(theme_key_problem("high-contrast"), None);
        assert_eq!(theme_key_problem("user.my-theme-2"), None);
        assert!(theme_key_problem("High Contrast").is_some());
        assert!(theme_key_problem("").is_some());
    }

    #[test]
    fn messages_are_assembled_and_truncated() {
        let message = Message::new()
            .push("alias cycle")
            .push(": ")
            .push("semantic.a");
        assert_eq!(message.as_str(), "alias cycle: semantic.a");
        let long = Message::new().push(&"x".repeat(300));
        assert_eq!(long.as_str().len(), 240);
    }
}

//! Customisation by feature: every token of the design system in nested panels. Features first
//! (controls, surfaces, editors, intents, palettes and scales: [`koebi_tokens::FEATURES`]), then
//! their groups (a component, an intent, a palette), then the parts of a group's paths several
//! of its tokens share (`background`, `border`), then the tokens, each an option editing its
//! value for the application and for the current theme ([`super::super::options`]).
//!
//! Every header ends with the dot of what changed below it: a click brings back every change,
//! the secondary button chooses the scope.

use koebi_core::{Rect, Vec2};
use koebi_design::schema::{Group, TokenInfo};
use koebi_design::{DesignSystem, TokenId};
use koebi_tokens::{FEATURES, Feature};
use koebi_ui::{Align, CollapsiblePanel, CollapsiblePanelState, MenuState, Ui, WidgetId};

use super::super::options::{self, OptionDesc, OptionRow};
use super::super::{Stack, UNBOUNDED};

/// What a folder holds, in the order of the schema.
#[derive(Debug)]
enum Child {
    Folder(Folder),
    Row(OptionRow),
}

/// A panel of the tree: a feature, a group, or a part of a group's paths.
#[derive(Debug)]
pub(super) struct Folder {
    title: String,
    /// What the folder is about, shown at the top of its content.
    description: Option<&'static str>,
    panel: CollapsiblePanelState,
    /// The menu of the header's dot.
    menu: MenuState,
    /// Every token below, for the header's dot.
    tokens: Vec<TokenId>,
    children: Vec<Child>,
}

impl Folder {
    fn new(title: String, description: Option<&'static str>) -> Self {
        Self {
            title,
            description,
            panel: CollapsiblePanelState::new(false),
            menu: MenuState::new(),
            tokens: Vec::new(),
            children: Vec::new(),
        }
    }

    /// The folder of `feature`: one folder per group.
    fn feature(design: &DesignSystem, feature: &Feature) -> Self {
        let mut folder = Self::new(feature.title.to_owned(), Some(feature.description));
        for group in feature.groups {
            let child = Self::group(design, group);
            folder.tokens.extend_from_slice(&child.tokens);
            folder.children.push(Child::Folder(child));
        }
        folder
    }

    /// The folder of `group`: its tokens, those sharing the first part of their path past the
    /// group's own name in a folder of that part, named by the rest.
    fn group(design: &DesignSystem, group: &Group) -> Self {
        let mut folder = Self::new(title_of(group.name()), None);
        // Each path past the group's own name (`color-picker.`), or past the part every path
        // shares.
        let slug = group.name().replace('_', "-").to_lowercase();
        let shared_part = shared_prefix(group.tokens().iter().map(TokenInfo::path));
        let rests: Vec<(TokenId, &'static str)> = group
            .tokens()
            .iter()
            .zip(group.start()..)
            .map(|(info, index)| {
                let path = info.path();
                let rest = path
                    .strip_prefix(slug.as_str())
                    .and_then(|rest| rest.strip_prefix('.'))
                    .or_else(|| path.get(shared_part..))
                    .filter(|rest| !rest.is_empty())
                    .unwrap_or(path);
                (TokenId::from_index(index), rest)
            })
            .collect();
        for &(token, rest) in &rests {
            folder.tokens.push(token);
            let part = rest.split_once('.').map(|(part, _)| part);
            let shared = part.filter(|part| {
                rests
                    .iter()
                    .filter(|(_, other)| {
                        other
                            .split_once('.')
                            .is_some_and(|(first, _)| first == *part)
                    })
                    .count()
                    > 1
            });
            let Some(part) = shared else {
                let row = OptionRow::new(OptionDesc::of(design, token, rest));
                folder.children.push(Child::Row(row));
                continue;
            };
            let label = &rest[part.len() + 1..];
            let row = Child::Row(OptionRow::new(OptionDesc::of(design, token, label)));
            let existing = folder.children.iter_mut().find_map(|child| match child {
                Child::Folder(sub) if sub.title == part => Some(sub),
                _ => None,
            });
            if let Some(sub) = existing {
                sub.tokens.push(token);
                sub.children.push(row);
            } else {
                let mut sub = Self::new(part.to_owned(), None);
                sub.tokens.push(token);
                sub.children.push(row);
                folder.children.push(Child::Folder(sub));
            }
        }
        folder
    }

    /// Shows the folder at the top of `area`, its menus within `bounds`. Returns its height.
    fn show(
        &mut self,
        ui: &mut Ui<'_>,
        area: Rect,
        id: WidgetId,
        design: &mut DesignSystem,
        bounds: Rect,
    ) -> f32 {
        let Self {
            title,
            description,
            panel,
            menu,
            tokens,
            children,
        } = self;
        let slot = options::reset_slot(design);
        let shown =
            CollapsiblePanel::new(id, panel, title)
                .trailing(slot)
                .show(ui, area, |ui, rect| {
                    let mut y = rect.min.y;
                    if let Some(description) = description {
                        let padding = ui.styles().panel.padding;
                        let font = ui.styles().text.body.clone();
                        let color = ui.styles().text.secondary;
                        let row = ui.styles().button.height;
                        let line = Rect::from_min_max(
                            Vec2::new(rect.min.x + padding, y),
                            Vec2::new(rect.max.x - padding, y + row),
                        );
                        ui.label(description, line, &font, color, Align::Start);
                        y += row;
                    }
                    for (index, child) in children.iter_mut().enumerate() {
                        let at = Rect::from_min_max(Vec2::new(rect.min.x, y), rect.max);
                        let key = id.with(index as u64 + 1);
                        y += match child {
                            Child::Folder(folder) => folder.show(ui, at, key, design, bounds),
                            Child::Row(row) => {
                                options::show_option(ui, at, key, design, row, bounds)
                            }
                        };
                    }
                    y - rect.min.y
                });
        if let Some(trailing) = shown.trailing {
            options::header_dot(ui, trailing, id.with(0), design, tokens, menu, bounds);
        }
        shown.height
    }

    /// Every option below, depth first.
    #[cfg(test)]
    fn rows(&self) -> Vec<&OptionRow> {
        let mut rows = Vec::new();
        for child in &self.children {
            match child {
                Child::Folder(folder) => rows.extend(folder.rows()),
                Child::Row(row) => rows.push(row),
            }
        }
        rows
    }

    /// The folder titled `title` among the children.
    #[cfg(test)]
    pub(super) fn child(&mut self, title: &str) -> Option<&mut Self> {
        self.children.iter_mut().find_map(|child| match child {
            Child::Folder(folder) if folder.title == title => Some(folder),
            _ => None,
        })
    }

    /// Every token below.
    #[cfg(test)]
    pub(super) fn tokens_below(&self) -> &[TokenId] {
        &self.tokens
    }

    /// Opens or closes the folder.
    #[cfg(test)]
    pub(super) fn set_open(&mut self, open: bool) {
        self.panel.set_open(open);
    }

    /// The option of `token` among the children.
    #[cfg(test)]
    pub(super) fn row(&mut self, token: TokenId) -> Option<&mut OptionRow> {
        self.children.iter_mut().find_map(|child| match child {
            Child::Row(row) if row.desc().token == token => Some(row),
            _ => None,
        })
    }
}

/// The tree of every token by feature.
#[derive(Debug)]
pub(super) struct FeatureTree {
    features: Vec<Folder>,
}

impl FeatureTree {
    pub(super) fn new(design: &DesignSystem) -> Self {
        Self {
            features: FEATURES
                .iter()
                .map(|feature| Folder::feature(design, feature))
                .collect(),
        }
    }

    /// Shows the features below what `stack` holds.
    pub(super) fn show(&mut self, ui: &mut Ui<'_>, stack: &mut Stack, design: &mut DesignSystem) {
        const ID: WidgetId = WidgetId::new("customisation-features");
        let bounds = stack.bounds;
        for (index, feature) in self.features.iter_mut().enumerate() {
            let area = Rect::from_min_max(
                Vec2::new(stack.left, stack.y),
                Vec2::new(stack.right, stack.y + UNBOUNDED),
            );
            let height = feature.show(ui, area, ID.with(index as u64), design, bounds);
            stack.y += height + stack.gap;
        }
    }

    /// The feature titled `title`.
    #[cfg(test)]
    pub(super) fn feature(&mut self, title: &str) -> Option<&mut Folder> {
        self.features
            .iter_mut()
            .find(|folder| folder.title == title)
    }

    /// Every token the tree edits, in its order.
    #[cfg(test)]
    pub(super) fn tokens(&self) -> Vec<TokenId> {
        self.features
            .iter()
            .flat_map(Folder::rows)
            .map(|row| row.desc().token)
            .collect()
    }
}

/// A group's name for the user: `COLOR_PICKER` becomes "Color picker".
fn title_of(name: &str) -> String {
    let mut title = name.replace('_', " ").to_lowercase();
    if let Some(first) = title.get(..1) {
        let upper = first.to_uppercase();
        title.replace_range(..1, &upper);
    }
    title
}

/// The length of the first part of the paths every one of `paths` starts with, its dot
/// included (`button.`), or 0.
fn shared_prefix<'a>(mut paths: impl Iterator<Item = &'a str>) -> usize {
    let Some(first) = paths.next() else {
        return 0;
    };
    let Some((part, _)) = first.split_once('.') else {
        return 0;
    };
    let prefix = part.len() + 1;
    if paths.all(|path| {
        path.len() > prefix && path.as_bytes().get(..prefix) == first.as_bytes().get(..prefix)
    }) {
        prefix
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use koebi_tokens::KOEBI;

    use super::*;

    #[test]
    fn names_and_shared_parts() {
        assert_eq!(title_of("COLOR_PICKER"), "Color picker");
        assert_eq!(title_of("ZONE"), "Zone");
        let paths = ["button.background.default", "button.border.width"];
        assert_eq!(shared_prefix(paths.into_iter()), "button.".len());
        assert_eq!(shared_prefix(["color.blue", "size.m"].into_iter()), 0);
        assert_eq!(shared_prefix(["alone"].into_iter()), 0);
    }

    #[test]
    fn the_tree_holds_every_token_once() {
        let design = DesignSystem::new(&KOEBI).unwrap();
        let tree = FeatureTree::new(&design);
        let mut tokens = tree.tokens();
        assert_eq!(tokens.len(), KOEBI.len());
        tokens.sort_by_key(|token| token.index());
        tokens.dedup();
        assert_eq!(tokens.len(), KOEBI.len());
    }

    #[test]
    fn groups_fold_the_parts_their_tokens_share() {
        let design = DesignSystem::new(&KOEBI).unwrap();
        let mut tree = FeatureTree::new(&design);
        let controls = tree.feature("Controls").unwrap();
        let picker = controls.child("Color picker").unwrap();
        // `color-picker.marker.size.default`, `.color.default`...: one folder, rows named by
        // the rest of their path.
        let marker = picker.child("marker").unwrap();
        let labels: Vec<&str> = marker.rows().iter().map(|row| row.desc().label).collect();
        assert!(labels.contains(&"size.default"), "{labels:?}");
        // A part one token alone has stays a row.
        assert!(picker.child("fine-factor").is_none());
    }
}

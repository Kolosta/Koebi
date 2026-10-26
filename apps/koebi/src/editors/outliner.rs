//! The Outliner template: the tree of a document in the list and tree view, with folding,
//! selection and reordering by drag and drop.
//!
//! The document is a template until Koebi has documents: groups holding layers, each layer
//! with its color. Every outliner shows the same tree; each keeps its own folds, selection
//! and scroll position.

use koebi_core::input::Key;
use koebi_core::{Color, Rect};
use koebi_input::{ActionDef, ActionGroup, Context, Shortcut};
use koebi_ui::{DropPlace, ListDelegate, ListItem, ListOptions, ListState, Ui, WidgetId};

/// The outliner: its keys apply where the pointer is over one.
pub(crate) const OUTLINER: Context = Context::new("outliner");

/// Deletes the selected rows.
pub(crate) static DELETE: ActionDef = ActionDef {
    id: "outliner.delete",
    label: "Delete",
    description: "Deletes the selected rows and what they hold.",
    context: OUTLINER,
    defaults: &[Shortcut::key(Key::X), Shortcut::key(Key::Delete)],
};

/// Puts the selected rows in a new group.
pub(crate) static GROUP: ActionDef = ActionDef {
    id: "outliner.group",
    label: "Group",
    description: "Puts the selected rows in a new group, where the first of them was.",
    context: OUTLINER,
    defaults: &[Shortcut::key(Key::G).primary()],
};

/// Unfolds every group.
pub(crate) static UNFOLD_ALL: ActionDef = ActionDef {
    id: "outliner.unfold-all",
    label: "Unfold all",
    description: "Unfolds every group of the outliner.",
    context: OUTLINER,
    defaults: &[Shortcut::key(Key::KeypadPlus)],
};

/// Folds every group.
pub(crate) static FOLD_ALL: ActionDef = ActionDef {
    id: "outliner.fold-all",
    label: "Fold all",
    description: "Folds every group of the outliner.",
    context: OUTLINER,
    defaults: &[Shortcut::key(Key::KeypadMinus)],
};

/// The outliner's actions, for the keymap.
pub(crate) static ACTIONS: ActionGroup = ActionGroup {
    name: "Outliner",
    actions: &[&DELETE, &GROUP, &UNFOLD_ALL, &FOLD_ALL],
};

/// A row of the document.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Node {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) depth: u16,
    /// Groups hold other rows; layers do not.
    pub(crate) group: bool,
    /// A layer's color.
    pub(crate) color: Option<Color>,
}

/// The document's tree: its rows in display order, depth first.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tree {
    nodes: Vec<Node>,
    next_id: u64,
}

/// The index after the subtree of the node at `index`.
fn subtree_end(nodes: &[Node], index: usize) -> usize {
    let depth = nodes[index].depth;
    nodes[index + 1..]
        .iter()
        .position(|node| node.depth <= depth)
        .map_or(nodes.len(), |offset| index + 1 + offset)
}

impl Tree {
    /// The template document.
    pub(crate) fn template() -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            next_id: 1,
        };
        let layers: [(&str, u16, Option<u32>); 10] = [
            ("Document", 0, None),
            ("Background", 1, Some(0x2D_9C_DB)),
            ("Sketch", 1, None),
            ("Construction", 2, Some(0x9B_9B_9B)),
            ("Gesture", 2, Some(0xF2_C9_4C)),
            ("Ink", 1, Some(0xBB_6B_D9)),
            ("Colors", 1, None),
            ("Flats", 2, Some(0xE0_4F_5F)),
            ("Shadows", 2, Some(0x6F_4F_A0)),
            ("Notes", 0, Some(0x27_AE_60)),
        ];
        for (name, depth, color) in layers {
            tree.push(name, depth, color.map(Color::from_hex));
        }
        tree
    }

    fn push(&mut self, name: &str, depth: u16, color: Option<Color>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(Node {
            id,
            name: name.to_owned(),
            depth,
            group: color.is_none(),
            color,
        });
        id
    }

    fn index(&self, id: u64) -> Option<usize> {
        self.nodes.iter().position(|node| node.id == id)
    }

    /// Whether `id` exists.
    pub(crate) fn contains(&self, id: u64) -> bool {
        self.index(id).is_some()
    }

    /// Fills `items` with the rows of the tree.
    pub(crate) fn items<'a>(&'a self, items: &mut Vec<ListItem<'a>>) {
        items.clear();
        items.extend(self.nodes.iter().map(|node| {
            let item = ListItem::new(node.id, &node.name).with_depth(node.depth);
            match node.color {
                Some(color) => item.with_color(color),
                None => item,
            }
        }));
    }

    /// Takes the subtrees of `ids` out of the tree, in display order, each moved to `depth`
    /// (its root there, its rows below it as before).
    fn take(&mut self, ids: &[u64], depth: u16) -> Vec<Node> {
        let mut taken = Vec::new();
        let mut index = 0;
        while index < self.nodes.len() {
            if ids.contains(&self.nodes[index].id) {
                let end = subtree_end(&self.nodes, index);
                let root = self.nodes[index].depth;
                taken.extend(self.nodes.drain(index..end).map(|mut node| {
                    node.depth = node.depth - root + depth;
                    node
                }));
            } else {
                index += 1;
            }
        }
        taken
    }

    /// Moves the subtrees of `ids` (topmost rows, in display order) to `place`. Returns
    /// whether the tree changed: a place inside the moved rows, or gone, moves nothing.
    pub(crate) fn move_rows(&mut self, ids: &[u64], place: DropPlace) -> bool {
        let (DropPlace::Before(target) | DropPlace::After(target) | DropPlace::Into(target)) =
            place;
        let Some(target_index) = self.index(target) else {
            return false;
        };
        // A target inside a moved subtree would lose its place.
        let inside = ids
            .iter()
            .filter_map(|id| self.index(*id))
            .any(|index| (index..subtree_end(&self.nodes, index)).contains(&target_index));
        if inside || ids.is_empty() {
            return false;
        }
        let target_depth = self.nodes[target_index].depth;
        let depth = match place {
            DropPlace::Into(_) => target_depth + 1,
            DropPlace::Before(_) | DropPlace::After(_) => target_depth,
        };
        let moved = self.take(ids, depth);
        if moved.is_empty() {
            return false;
        }
        let Some(target_index) = self.index(target) else {
            return false;
        };
        let at = match place {
            DropPlace::Before(_) => target_index,
            DropPlace::After(_) | DropPlace::Into(_) => subtree_end(&self.nodes, target_index),
        };
        self.nodes.splice(at..at, moved);
        true
    }

    /// Deletes the subtrees of `ids`. Returns whether something went.
    pub(crate) fn delete(&mut self, ids: &[u64]) -> bool {
        !self.take(ids, 0).is_empty()
    }

    /// Puts the subtrees of `ids` (topmost rows, in display order) in a new group where the
    /// first of them was. Returns the group.
    pub(crate) fn group(&mut self, ids: &[u64]) -> Option<u64> {
        let first = *ids.first()?;
        let index = self.index(first)?;
        let depth = self.nodes[index].depth;
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(
            index,
            Node {
                id,
                name: "Group".to_owned(),
                depth,
                group: true,
                color: None,
            },
        );
        let moved = self.take(ids, depth + 1);
        // The group ends where its rows end: after its new rows.
        let at = self.index(id)? + 1;
        self.nodes.splice(at..at, moved);
        Some(id)
    }
}

/// Where rows may go: into groups only.
struct Rules<'a> {
    tree: &'a Tree,
}

impl ListDelegate for Rules<'_> {
    fn can_drop(&self, _dragged: &[u64], place: DropPlace) -> bool {
        match place {
            DropPlace::Into(id) => self
                .tree
                .nodes
                .iter()
                .any(|node| node.id == id && node.group),
            DropPlace::Before(_) | DropPlace::After(_) => true,
        }
    }
}

/// The topmost selected rows, in display order.
fn selected_roots(tree: &Tree, state: &ListState, roots: &mut Vec<u64>) {
    roots.clear();
    let mut skip_below = None;
    for node in &tree.nodes {
        if skip_below.is_some_and(|depth| node.depth > depth) {
            continue;
        }
        skip_below = None;
        if state.is_selected(node.id) {
            roots.push(node.id);
            skip_below = Some(node.depth);
        }
    }
}

/// Draws an outliner showing `tree` in `rect`, with its own `state`, and applies its edits:
/// drops, the outliner's actions. `items` is reused from frame to frame.
pub(crate) fn show(
    ui: &mut Ui<'_>,
    id: WidgetId,
    rect: Rect,
    tree: &mut Tree,
    state: &mut ListState,
    items: &mut Vec<ListItem<'static>>,
) {
    let pointer_over = ui
        .input()
        .pointer()
        .is_some_and(|point| rect.contains(point));
    let mut rows = super::recycle(core::mem::take(items));
    tree.items(&mut rows);
    let response = ui.zone(rect, OUTLINER, |ui| {
        ui.list(
            id,
            rect,
            state,
            &rows,
            &ListOptions::TREE,
            &mut Rules { tree },
        )
    });
    *items = super::recycle(rows);
    if let Some(place) = response.dropped {
        let dragged = state.dragged().to_vec();
        if tree.move_rows(&dragged, place) {
            if let DropPlace::Into(group) = place {
                state.set_folded(group, false);
            }
            koebi_core::info!("moved {} row(s)", dragged.len());
        }
    }
    if !pointer_over || state.is_dragging() {
        return;
    }
    let mut roots = Vec::new();
    if ui.triggered(&DELETE) {
        selected_roots(tree, state, &mut roots);
        if tree.delete(&roots) {
            state.retain(|id| tree.contains(id));
        }
    }
    if ui.triggered(&GROUP) {
        selected_roots(tree, state, &mut roots);
        if let Some(group) = tree.group(&roots) {
            state.clear_selection();
            state.set_active(Some(group));
        }
    }
    let fold = ui.triggered(&FOLD_ALL);
    if fold || ui.triggered(&UNFOLD_ALL) {
        for node in tree.nodes.iter().filter(|node| node.group) {
            state.set_folded(node.id, fold);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(tree: &Tree) -> Vec<(String, u16)> {
        tree.nodes
            .iter()
            .map(|node| (node.name.clone(), node.depth))
            .collect()
    }

    fn id(tree: &Tree, name: &str) -> u64 {
        tree.nodes.iter().find(|node| node.name == name).unwrap().id
    }

    fn pairs(list: &[(&str, u16)]) -> Vec<(String, u16)> {
        list.iter()
            .map(|(name, depth)| ((*name).to_owned(), *depth))
            .collect()
    }

    #[test]
    fn rows_move_with_their_subtree_to_their_new_level() {
        let mut tree = Tree::template();
        // The Sketch group into Colors, after Shadows.
        let (sketch, colors) = (id(&tree, "Sketch"), id(&tree, "Colors"));
        assert!(tree.move_rows(&[sketch], DropPlace::Into(colors)));
        assert_eq!(
            names(&tree),
            pairs(&[
                ("Document", 0),
                ("Background", 1),
                ("Ink", 1),
                ("Colors", 1),
                ("Flats", 2),
                ("Shadows", 2),
                ("Sketch", 2),
                ("Construction", 3),
                ("Gesture", 3),
                ("Notes", 0),
            ])
        );
        // Notes before Document; Ink after Document, at the top level.
        let (notes, document, ink) = (id(&tree, "Notes"), id(&tree, "Document"), id(&tree, "Ink"));
        assert!(tree.move_rows(&[notes], DropPlace::Before(document)));
        assert!(tree.move_rows(&[ink], DropPlace::After(document)));
        assert_eq!(names(&tree)[0], ("Notes".to_owned(), 0));
        assert_eq!(names(&tree).last().unwrap(), &("Ink".to_owned(), 0));
    }

    #[test]
    fn rows_never_move_inside_themselves() {
        let mut tree = Tree::template();
        let before = tree.clone();
        let (sketch, gesture) = (id(&tree, "Sketch"), id(&tree, "Gesture"));
        assert!(!tree.move_rows(&[sketch], DropPlace::After(gesture)));
        assert!(!tree.move_rows(&[sketch], DropPlace::Into(sketch)));
        assert!(!tree.move_rows(&[sketch], DropPlace::Before(999)));
        assert_eq!(tree, before);
    }

    #[test]
    fn groups_take_the_selection_and_deletes_take_subtrees() {
        let mut tree = Tree::template();
        let (ink, notes) = (id(&tree, "Ink"), id(&tree, "Notes"));
        let group = tree.group(&[ink, notes]).unwrap();
        let index = tree.index(group).unwrap();
        assert_eq!(tree.nodes[index].depth, 1);
        assert_eq!(
            names(&tree)[index..index + 3],
            pairs(&[("Group", 1), ("Ink", 2), ("Notes", 2)])
        );
        assert!(tree.delete(&[id(&tree, "Sketch")]));
        assert!(!tree.contains(id(&Tree::template(), "Gesture")));
        assert!(!tree.delete(&[999]));
    }

    #[test]
    fn only_groups_take_rows_in() {
        let tree = Tree::template();
        let rules = Rules { tree: &tree };
        assert!(rules.can_drop(&[], DropPlace::Into(id(&tree, "Colors"))));
        assert!(!rules.can_drop(&[], DropPlace::Into(id(&tree, "Ink"))));
        assert!(rules.can_drop(&[], DropPlace::After(id(&tree, "Ink"))));
    }
}

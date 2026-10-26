//! The application's editors: what each area of the workspace shows.
//!
//! Each editor keeps a state for every kind it has shown: switching its type and back finds
//! it as it was. The gallery shows the component sheet; the other kinds are templates:
//! [`viewport`] (a canvas that pans and zooms, rulers), [`outliner`] and [`info`] (on the list
//! and tree view) and [`timeline`] (scrolling both ways).

pub(crate) mod info;
pub(crate) mod outliner;
pub(crate) mod timeline;
pub(crate) mod viewport;

use koebi_core::Rect;
use koebi_design::DesignSystem;
use koebi_ui::{BarLayout, EditorHost, ListItem, ListState, Ui, WidgetId};
use koebi_workspace::{EditorId, GRID, Orientation, Workspace};

use crate::gallery::Gallery;
use crate::messages::Messages;
use outliner::Tree;
use timeline::Playhead;

/// An empty vector of rows on the allocation of `items`: rows borrow the data they show for
/// one frame, the allocation stays from frame to frame.
#[allow(
    clippy::unnecessary_filter_map,
    reason = "the rows change lifetime: only an empty collect can give them a new one"
)]
fn recycle<'a>(items: Vec<ListItem<'_>>) -> Vec<ListItem<'a>> {
    // Collecting nothing from the vector's own iterator reuses its allocation.
    items.into_iter().filter_map(|_| None).collect()
}

/// What an editor shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum EditorKind {
    /// The component sheet and the token editor.
    Gallery,
    /// The canvas.
    Viewport,
    /// The tree of the document.
    Outliner,
    /// Messages.
    Info,
    /// Time.
    Timeline,
}

impl EditorKind {
    /// Every kind, in the order of the editor type dropdown.
    pub(crate) const ALL: [Self; 5] = [
        Self::Gallery,
        Self::Viewport,
        Self::Outliner,
        Self::Info,
        Self::Timeline,
    ];

    /// The titles of [`EditorKind::ALL`], for the dropdown.
    const TITLES: [&'static str; 5] = ["Gallery", "Viewport", "Outliner", "Info", "Timeline"];

    /// The title shown in the editor's tab and type dropdown.
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Gallery => "Gallery",
            Self::Viewport => "Viewport",
            Self::Outliner => "Outliner",
            Self::Info => "Info",
            Self::Timeline => "Timeline",
        }
    }
}

/// An editor: its kind, and the state of each kind it has shown.
#[derive(Clone, Debug)]
struct Entry {
    id: EditorId,
    kind: EditorKind,
    viewport: viewport::View,
    outliner: ListState,
    info: ListState,
    timeline: timeline::View,
}

impl Entry {
    fn new(id: EditorId, kind: EditorKind) -> Self {
        Self {
            id,
            kind,
            viewport: viewport::View::default(),
            outliner: ListState::new(),
            info: ListState::new(),
            timeline: timeline::View::default(),
        }
    }
}

/// Every editor of the workspace.
#[derive(Clone, Debug, Default)]
pub(crate) struct Editors {
    entries: Vec<Entry>,
}

impl Editors {
    /// The kind of `editor`; editors the application never registered show as viewports.
    pub(crate) fn kind(&self, editor: EditorId) -> EditorKind {
        self.entries
            .iter()
            .find(|entry| entry.id == editor)
            .map_or(EditorKind::Viewport, |entry| entry.kind)
    }

    /// Sets the kind of `editor`; the states of its other kinds stay.
    pub(crate) fn set(&mut self, editor: EditorId, kind: EditorKind) {
        self.entry(editor).kind = kind;
    }

    /// The entry of `editor`, created as a viewport when missing.
    fn entry(&mut self, editor: EditorId) -> &mut Entry {
        let found = self.entries.iter().position(|entry| entry.id == editor);
        let index = found.unwrap_or(self.entries.len());
        if found.is_none() {
            self.entries.push(Entry::new(editor, EditorKind::Viewport));
        }
        &mut self.entries[index]
    }

    /// `editor` becomes a copy of `source`: its kind and the state of every kind.
    fn copy(&mut self, source: EditorId, editor: EditorId) {
        let mut copy = self.entry(source).clone();
        copy.id = editor;
        *self.entry(editor) = copy;
    }

    fn remove(&mut self, editors: &[EditorId]) {
        self.entries.retain(|entry| !editors.contains(&entry.id));
    }
}

/// The workspace Koebi opens with: the gallery, the outliner and info on the right, the
/// timeline below.
pub(crate) fn default_workspace() -> (Workspace, Editors) {
    let mut workspace = Workspace::new();
    let mut editors = Editors::default();
    let Some((main, _)) = workspace.layout().areas().next() else {
        return (workspace, editors);
    };
    if let Some(editor) = workspace.stack(main).map(koebi_workspace::Stack::active) {
        editors.set(editor, EditorKind::Gallery);
    }
    let fraction = |numerator: u32, denominator: u32| GRID / denominator * numerator;
    let splits = [
        (
            main,
            Orientation::Vertical,
            fraction(18, 25),
            EditorKind::Outliner,
        ),
        (
            main,
            Orientation::Horizontal,
            fraction(4, 5),
            EditorKind::Timeline,
        ),
    ];
    let mut right = None;
    for (area, orientation, position, kind) in splits {
        if let Ok(split) = workspace.split(area, orientation, position, 1) {
            editors.set(split.editor, kind);
            right = right.or(Some(split.area));
        }
    }
    if let Some(right) = right
        && let Ok(split) = workspace.split(right, Orientation::Horizontal, fraction(11, 20), 1)
    {
        editors.set(split.editor, EditorKind::Info);
    }
    (workspace, editors)
}

/// What the editors show, and what they reuse from frame to frame.
#[derive(Debug)]
pub(crate) struct Content {
    pub(crate) gallery: Gallery,
    /// The document the outliners show.
    pub(crate) tree: Tree,
    /// The messages the Info editors show.
    pub(crate) messages: Messages,
    /// The frames the timelines show, and the current one.
    pub(crate) playhead: Playhead,
    /// The rows of the list being drawn.
    items: Vec<ListItem<'static>>,
}

impl Content {
    pub(crate) fn new() -> Self {
        Self {
            gallery: Gallery::new(),
            tree: Tree::template(),
            messages: Messages::new(),
            playhead: Playhead::new(),
            items: Vec::new(),
        }
    }
}

/// The editors as the workspace view sees them, for one frame.
pub(crate) struct Host<'a> {
    pub(crate) editors: &'a mut Editors,
    pub(crate) content: &'a mut Content,
    pub(crate) design: &'a mut DesignSystem,
    /// The window, where the gallery's menus may open.
    pub(crate) window: Rect,
}

impl EditorHost for Host<'_> {
    fn title(&self, editor: EditorId) -> &str {
        self.editors.kind(editor).title()
    }

    fn kinds(&self) -> &[&str] {
        &EditorKind::TITLES
    }

    fn kind(&self, editor: EditorId) -> usize {
        let kind = self.editors.kind(editor);
        EditorKind::ALL
            .iter()
            .position(|candidate| *candidate == kind)
            .unwrap_or(0)
    }

    fn set_kind(&mut self, editor: EditorId, kind: usize) {
        if let Some(kind) = EditorKind::ALL.get(kind) {
            self.editors.set(editor, *kind);
        }
    }

    fn content(&mut self, ui: &mut Ui<'_>, editor: EditorId, rect: Rect) {
        let id = WidgetId::new("editor").with(u64::from(editor.get()));
        let content = &mut *self.content;
        let entry = self.editors.entry(editor);
        match entry.kind {
            EditorKind::Gallery => content.gallery.show(ui, self.design, rect, self.window),
            EditorKind::Outliner => outliner::show(
                ui,
                id,
                rect,
                &mut content.tree,
                &mut entry.outliner,
                &mut content.items,
            ),
            EditorKind::Info => info::show(
                ui,
                id,
                rect,
                self.design,
                &mut content.messages,
                &mut entry.info,
                &mut content.items,
            ),
            EditorKind::Viewport => viewport::show(ui, rect, self.design, &mut entry.viewport),
            EditorKind::Timeline => timeline::show(
                ui,
                id,
                rect,
                self.design,
                &mut content.playhead,
                &mut entry.timeline,
            ),
        }
    }

    fn top_bar(&mut self, ui: &mut Ui<'_>, editor: EditorId, mut bar: BarLayout) {
        let id = WidgetId::new("editor").with(u64::from(editor.get()));
        let entry = self.editors.entry(editor);
        if entry.kind == EditorKind::Viewport {
            viewport::top_bar(ui, id, &mut entry.viewport, &mut bar);
        }
    }

    fn copied(&mut self, source: EditorId, editor: EditorId) {
        self.editors.copy(source, editor);
    }

    fn removed(&mut self, editors: &[EditorId]) {
        self.editors.remove(editors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_workspace_holds_every_kind_of_editor() {
        let (workspace, editors) = default_workspace();
        workspace.check().unwrap();
        assert_eq!(workspace.layout().len(), 4);
        let mut kinds: Vec<_> = workspace
            .editors()
            .map(|editor| editors.kind(editor))
            .collect();
        kinds.sort_by_key(|kind| kind.title());
        assert_eq!(
            kinds,
            [
                EditorKind::Gallery,
                EditorKind::Info,
                EditorKind::Outliner,
                EditorKind::Timeline
            ]
        );
    }

    #[test]
    fn the_dropdown_lists_every_kind_by_its_title() {
        assert!(
            EditorKind::ALL
                .iter()
                .zip(EditorKind::TITLES)
                .all(|(kind, title)| kind.title() == title)
        );
    }

    #[test]
    fn copies_keep_their_kind_and_removed_editors_go() {
        let mut editors = Editors::default();
        let (a, b) = (EditorId::from_saved(1), EditorId::from_saved(2));
        editors.set(a, EditorKind::Outliner);
        let mut content = Content::new();
        let mut design = DesignSystem::new(&koebi_tokens::KOEBI).expect("Koebi's schema is valid");
        let mut host = Host {
            editors: &mut editors,
            content: &mut content,
            design: &mut design,
            window: Rect::ZERO,
        };
        host.copied(a, b);
        assert_eq!(host.title(b), "Outliner");
        assert_eq!(host.kind(b), 2);
        host.set_kind(b, 4);
        assert_eq!(host.title(b), "Timeline");
        host.removed(&[a]);
        assert_eq!(editors.kind(b), EditorKind::Timeline);
        assert_eq!(editors.entries.len(), 1);
    }

    #[test]
    fn editors_keep_the_state_of_each_kind_they_showed() {
        let mut editors = Editors::default();
        let (a, b) = (EditorId::from_saved(1), EditorId::from_saved(2));
        editors.set(a, EditorKind::Outliner);
        editors.entry(a).outliner.set_folded(7, true);
        editors.set(a, EditorKind::Info);
        editors.set(a, EditorKind::Outliner);
        assert!(editors.entry(a).outliner.is_folded(7));
        // A split copies the state too.
        editors.copy(a, b);
        assert_eq!(editors.kind(b), EditorKind::Outliner);
        assert!(editors.entry(b).outliner.is_folded(7));
    }
}

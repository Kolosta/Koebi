//! The application's editors: what each area of the workspace shows.
//!
//! Each editor keeps a state for every kind it has shown: switching its type and back finds
//! it as it was. The gallery shows the component sheet; the other kinds are templates:
//! [`viewport`] (a canvas that pans and zooms, rulers), [`outliner`] and [`info`] (on the list
//! and tree view), [`timeline`] (scrolling both ways) and [`properties`] (the components in
//! real use, editing the document's object).

pub(crate) mod info;
pub(crate) mod outliner;
pub(crate) mod properties;
pub(crate) mod saved;
pub(crate) mod timeline;
pub(crate) mod viewport;

use koebi_core::Rect;
use koebi_design::DesignSystem;
use koebi_i18n::tr;
use koebi_icons::IconId;
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
    /// The properties of the document's object.
    Properties,
}

impl EditorKind {
    /// Every kind, in the order of the editor type dropdown.
    pub(crate) const ALL: [Self; 6] = [
        Self::Gallery,
        Self::Viewport,
        Self::Outliner,
        Self::Properties,
        Self::Info,
        Self::Timeline,
    ];

    /// The titles of [`EditorKind::ALL`] in the current language, for the dropdown.
    pub(crate) fn titles() -> [&'static str; 6] {
        Self::ALL.map(Self::title)
    }

    /// The icons of [`EditorKind::ALL`], for the dropdown.
    pub(crate) const ICONS: [IconId; 6] = [
        koebi_icons::editors::GALLERY,
        koebi_icons::editors::VIEWPORT,
        koebi_icons::editors::OUTLINER,
        koebi_icons::editors::PROPERTIES,
        koebi_icons::editors::INFO,
        koebi_icons::editors::TIMELINE,
    ];

    /// The icon shown before the editor's title, in its tab and the type dropdown.
    pub(crate) const fn icon(self) -> IconId {
        match self {
            Self::Gallery => koebi_icons::editors::GALLERY,
            Self::Viewport => koebi_icons::editors::VIEWPORT,
            Self::Outliner => koebi_icons::editors::OUTLINER,
            Self::Properties => koebi_icons::editors::PROPERTIES,
            Self::Info => koebi_icons::editors::INFO,
            Self::Timeline => koebi_icons::editors::TIMELINE,
        }
    }

    /// The title shown in the editor's tab and type dropdown.
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Gallery => tr("Gallery"),
            Self::Viewport => tr("Viewport"),
            Self::Outliner => tr("Outliner"),
            Self::Info => tr("Info"),
            Self::Timeline => tr("Timeline"),
            Self::Properties => tr("Properties"),
        }
    }
}

/// An editor: its kind, and the state of each kind it has shown.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    id: EditorId,
    kind: EditorKind,
    viewport: viewport::View,
    outliner: ListState,
    info: ListState,
    timeline: timeline::View,
    properties: properties::View,
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
            properties: properties::View::default(),
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

    /// What `editor` is, to give it to another window's editors: its kind, and the state of every
    /// kind it showed.
    pub(crate) fn entry_of(&mut self, editor: EditorId) -> Entry {
        self.entry(editor).clone()
    }

    /// Forgets `editor`, which left for another window.
    pub(crate) fn remove_editor(&mut self, editor: EditorId) {
        self.remove(&[editor]);
    }

    /// `editor` becomes what `entry` was in another window.
    pub(crate) fn adopt(&mut self, editor: EditorId, mut entry: Entry) {
        entry.id = editor;
        *self.entry(editor) = entry;
    }
}

/// The workspace Koebi opens with: the gallery, the outliner, the properties and info on the
/// right, the timeline below.
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
        && let Ok(split) = workspace.split(right, Orientation::Horizontal, fraction(15, 20), 1)
    {
        editors.set(split.editor, EditorKind::Info);
    }
    if let Some(right) = right
        && let Ok(split) = workspace.split(right, Orientation::Horizontal, fraction(7, 20), 1)
    {
        editors.set(split.editor, EditorKind::Properties);
    }
    (workspace, editors)
}

/// The workspace a new floating window opens with: one viewport.
pub(crate) fn default_floating() -> (Workspace, Editors) {
    let workspace = Workspace::new();
    let mut editors = Editors::default();
    if let Some(editor) = workspace.editors().next() {
        editors.set(editor, EditorKind::Viewport);
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
    /// The object the Properties editors edit.
    pub(crate) object: properties::Object,
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
            object: properties::Object::template(),
            items: Vec::new(),
        }
    }
}

/// The editors as the workspace view sees them, for one frame.
pub(crate) struct Host<'a> {
    pub(crate) editors: &'a mut Editors,
    pub(crate) content: &'a mut Content,
    pub(crate) design: &'a mut DesignSystem,
    /// The window, where the editors' menus may open.
    pub(crate) window: Rect,
    /// [`EditorKind::titles`], read once a frame.
    pub(crate) titles: [&'static str; 6],
}

impl EditorHost for Host<'_> {
    fn title(&self, editor: EditorId) -> &str {
        self.editors.kind(editor).title()
    }

    fn kinds(&self) -> &[&str] {
        &self.titles
    }

    fn icon(&self, editor: EditorId) -> Option<IconId> {
        Some(self.editors.kind(editor).icon())
    }

    fn kind_icons(&self) -> &[IconId] {
        &EditorKind::ICONS
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
            EditorKind::Properties => properties::show(
                ui,
                id,
                rect,
                self.window,
                self.design,
                &mut content.object,
                &mut entry.properties,
            ),
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
        assert_eq!(workspace.layout().len(), 5);
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
                EditorKind::Properties,
                EditorKind::Timeline
            ]
        );
    }

    #[test]
    fn the_dropdown_lists_every_kind_by_its_title() {
        assert!(
            EditorKind::ALL
                .iter()
                .zip(EditorKind::titles())
                .all(|(kind, title)| kind.title() == title)
        );
        assert!(
            EditorKind::ALL
                .iter()
                .zip(EditorKind::ICONS)
                .all(|(kind, icon)| kind.icon() == icon)
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
            titles: EditorKind::titles(),
        };
        host.copied(a, b);
        assert_eq!(host.title(b), "Outliner");
        assert_eq!(host.kind(b), 2);
        host.set_kind(b, 5);
        assert_eq!(host.title(b), "Timeline");
        host.removed(&[a]);
        assert_eq!(editors.kind(b), EditorKind::Timeline);
        assert_eq!(editors.entries.len(), 1);
    }

    #[test]
    fn an_editor_keeps_its_kind_and_state_in_another_window() {
        let mut main = Editors::default();
        let (a, b) = (EditorId::from_saved(1), EditorId::from_saved(1));
        main.set(a, EditorKind::Outliner);
        main.entry(a).outliner.set_folded(7, true);
        let taken = main.entry_of(a);
        main.remove_editor(a);
        assert!(main.entries.is_empty());
        // The other window numbers its editors on its own: the same number is another editor.
        let (workspace, mut floating) = default_floating();
        assert_eq!(workspace.editors().count(), 1);
        floating.adopt(b, taken);
        assert_eq!(floating.kind(b), EditorKind::Outliner);
        assert!(floating.entry(b).outliner.is_folded(7));
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

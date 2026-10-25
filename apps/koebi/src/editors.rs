//! The application's editors: what each area of the workspace shows.
//!
//! The gallery shows the component sheet; the other kinds are placeholders until their
//! templates exist (#76).

use koebi_core::Rect;
use koebi_design::DesignSystem;
use koebi_ui::{Align, EditorHost, Ui};
use koebi_workspace::{EditorId, GRID, Orientation, Workspace};

use crate::gallery::Gallery;

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
    /// The title shown in the editor's top bar.
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

/// The kind of every editor of the workspace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Editors {
    kinds: Vec<(EditorId, EditorKind)>,
}

impl Editors {
    /// The kind of `editor`; editors the application never registered show as viewports.
    pub(crate) fn kind(&self, editor: EditorId) -> EditorKind {
        self.kinds
            .iter()
            .find(|(id, _)| *id == editor)
            .map_or(EditorKind::Viewport, |(_, kind)| *kind)
    }

    /// Sets the kind of `editor`.
    pub(crate) fn set(&mut self, editor: EditorId, kind: EditorKind) {
        match self.kinds.iter_mut().find(|(id, _)| *id == editor) {
            Some(entry) => entry.1 = kind,
            None => self.kinds.push((editor, kind)),
        }
    }

    fn remove(&mut self, editors: &[EditorId]) {
        self.kinds.retain(|(id, _)| !editors.contains(id));
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

/// The editors as the workspace view sees them, for one frame.
pub(crate) struct Host<'a> {
    pub(crate) editors: &'a mut Editors,
    pub(crate) gallery: &'a mut Gallery,
    pub(crate) design: &'a mut DesignSystem,
    /// The window, where the gallery's menus may open.
    pub(crate) window: Rect,
}

impl EditorHost for Host<'_> {
    fn title(&self, editor: EditorId) -> &str {
        self.editors.kind(editor).title()
    }

    fn content(&mut self, ui: &mut Ui<'_>, editor: EditorId, rect: Rect) {
        match self.editors.kind(editor) {
            EditorKind::Gallery => self.gallery.show(ui, self.design, rect, self.window),
            kind => {
                // A placeholder: the editor's name in the middle of its area.
                let styles = ui.styles();
                let (font, color) = (styles.text.body.clone(), styles.text.secondary);
                ui.label(kind.title(), rect, &font, color, Align::Center);
            }
        }
    }

    fn copied(&mut self, source: EditorId, editor: EditorId) {
        let kind = self.editors.kind(source);
        self.editors.set(editor, kind);
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
    fn copies_keep_their_kind_and_removed_editors_go() {
        let mut editors = Editors::default();
        let (a, b) = (EditorId::from_saved(1), EditorId::from_saved(2));
        editors.set(a, EditorKind::Outliner);
        let mut gallery = Gallery::new();
        let mut design = DesignSystem::new(&koebi_tokens::KOEBI).expect("Koebi's schema is valid");
        let mut host = Host {
            editors: &mut editors,
            gallery: &mut gallery,
            design: &mut design,
            window: Rect::ZERO,
        };
        host.copied(a, b);
        assert_eq!(host.title(b), "Outliner");
        host.removed(&[a]);
        assert_eq!(editors.kind(b), EditorKind::Outliner);
        assert_eq!(editors.kinds.len(), 1);
    }
}

//! The text form of the workspace: its areas, the editors of each area, and each editor's
//! kind, by stable names.
//!
//! ```text
//! # Koebi workspace 1
//! area 0 0 0 47185 52428
//! stack 0 0 0 4
//! editor 0 gallery
//! ```
//!
//! `area <id> <x0> <y0> <x1> <y1>` places an area on the grid, `stack <area> <shown> <editor>...`
//! lists its editors in tab order with the position of the one shown, `editor <id> <kind>`
//! gives an editor's kind. Unknown lines and kinds are skipped, so that files survive code
//! changes; a layout that does not tile the grid is refused.

use std::fmt::Write as _;

use koebi_workspace::{EditorId, GridRect, Layout, Workspace};

use super::{EditorKind, Editors};

/// The first line of a saved workspace.
pub(crate) const HEADER: &str = "# Koebi workspace 1";

impl EditorKind {
    /// The kind's stable name in saved files.
    fn key(self) -> &'static str {
        match self {
            Self::Gallery => "gallery",
            Self::Viewport => "viewport",
            Self::Outliner => "outliner",
            Self::Info => "info",
            Self::Timeline => "timeline",
            Self::Properties => "properties",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }
}

/// `workspace` and the kinds of its editors as text.
pub(crate) fn encode(workspace: &Workspace, editors: &Editors) -> String {
    let mut text = String::from(HEADER);
    text.push('\n');
    // Writing into a `String` cannot fail.
    for (area, rect) in workspace.layout().areas() {
        let _infallible = writeln!(
            text,
            "area {} {} {} {} {}",
            area.get(),
            rect.x0,
            rect.y0,
            rect.x1,
            rect.y1
        );
    }
    for (area, _) in workspace.layout().areas() {
        if let Some(stack) = workspace.stack(area) {
            let _infallible = write!(text, "stack {} {}", area.get(), stack.active_index());
            for editor in stack.editors() {
                let _infallible = write!(text, " {}", editor.get());
            }
            text.push('\n');
        }
    }
    for editor in workspace.editors() {
        let _infallible = writeln!(
            text,
            "editor {} {}",
            editor.get(),
            editors.kind(editor).key()
        );
    }
    text
}

/// The workspace and editors of `text`, and the lines skipped.
///
/// # Errors
///
/// When `text` is not a saved workspace, or its areas and stacks do not make one.
pub(crate) fn decode(text: &str) -> Result<(Workspace, Editors, Vec<String>), String> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    if lines.next() != Some(HEADER) {
        return Err(format!(
            "not a saved workspace: the first line must be {HEADER:?}"
        ));
    }
    let mut skipped = Vec::new();
    let mut areas = Vec::new();
    let mut stacks = Vec::new();
    let mut kinds = Vec::new();
    for line in lines {
        if line.starts_with('#') {
            continue;
        }
        let mut words = line.split_whitespace();
        let keyword = words.next().unwrap_or_default();
        let numbers: Option<Vec<u32>> = match keyword {
            "area" | "stack" => words.map(|word| word.parse().ok()).collect(),
            "editor" => {
                let id = words.next().and_then(|word| word.parse::<u32>().ok());
                match (id, words.next().and_then(EditorKind::from_key)) {
                    (Some(id), Some(kind)) => kinds.push((id, kind)),
                    _ => skipped.push(format!("unknown editor line: {line}")),
                }
                continue;
            }
            _ => {
                skipped.push(format!("unknown line: {line}"));
                continue;
            }
        };
        match (keyword, numbers.as_deref()) {
            ("area", Some(&[id, x0, y0, x1, y1])) => {
                areas.push((id, GridRect { x0, y0, x1, y1 }));
            }
            ("stack", Some([area, shown, editors @ ..])) if !editors.is_empty() => {
                let shown = usize::try_from(*shown).unwrap_or(usize::MAX);
                stacks.push((*area, editors.to_vec(), shown));
            }
            _ => return Err(format!("invalid line: {line}")),
        }
    }
    let layout = Layout::from_areas(&areas).map_err(|error| error.to_string())?;
    let workspace = Workspace::from_saved(layout, &stacks).map_err(|error| error.to_string())?;
    let mut editors = Editors::default();
    for editor in workspace.editors() {
        let kind = kinds
            .iter()
            .find(|(id, _)| *id == editor.get())
            .map_or(EditorKind::Viewport, |(_, kind)| *kind);
        editors.set(editor, kind);
    }
    Ok((workspace, editors, skipped))
}

/// Whether `editors` gives the editors of `workspace` the kinds of `saved`.
pub(crate) fn same_kinds(workspace: &Workspace, editors: &Editors, saved: &Editors) -> bool {
    workspace
        .editors()
        .all(|editor: EditorId| editors.kind(editor) == saved.kind(editor))
}

#[cfg(test)]
mod tests {
    use super::super::default_workspace;
    use super::*;

    #[test]
    fn workspaces_read_back_as_they_were_written() {
        let (workspace, editors) = default_workspace();
        let text = encode(&workspace, &editors);
        assert!(text.starts_with(HEADER));
        let (restored, kinds, skipped) = decode(&text).unwrap();
        assert_eq!(restored, workspace);
        assert!(same_kinds(&workspace, &kinds, &editors));
        assert_eq!(skipped, Vec::<String>::new());
    }

    #[test]
    fn unknown_lines_and_kinds_are_skipped_and_broken_layouts_refused() {
        let text =
            format!("{HEADER}\narea 0 0 0 65536 65536\nstack 0 0 3\neditor 3 sculpt\npanel x\n");
        let (workspace, editors, skipped) = decode(&text).unwrap();
        assert_eq!(workspace.layout().len(), 1);
        assert_eq!(editors.kind(EditorId::from_saved(3)), EditorKind::Viewport);
        assert_eq!(skipped.len(), 2);
        assert!(decode("area 0 0 0 1 1").is_err());
        assert!(decode(&format!("{HEADER}\narea 0 0 0 100 100\nstack 0 0 1\n")).is_err());
        assert!(decode(&format!("{HEADER}\narea 0 0 0 65536 65536\nstack 0 4 1\n")).is_err());
    }
}

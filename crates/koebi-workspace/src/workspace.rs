//! The workspace: a layout whose areas hold stacks of editors.

use std::fmt;

use crate::grid::{Orientation, Side};
use crate::layout::{AreaId, Layout, LayoutError, Line};

/// An editor shown in an area. Its state lives outside the workspace, keyed by this
/// identifier; identifiers are never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EditorId(pub(crate) u32);

impl EditorId {
    /// An identifier read back from saved data.
    pub fn from_saved(id: u32) -> Self {
        Self(id)
    }

    /// The identifier as a number, for persistence.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// The editors of an area, in tab order, one of them shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stack {
    editors: Vec<EditorId>,
    active: usize,
}

impl Stack {
    fn new(editor: EditorId) -> Self {
        Self {
            editors: vec![editor],
            active: 0,
        }
    }

    /// The editors, in tab order.
    pub fn editors(&self) -> &[EditorId] {
        &self.editors
    }

    /// The editor shown.
    pub fn active(&self) -> EditorId {
        self.editors[self.active]
    }

    /// Its position in the tabs.
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Number of editors (at least one).
    pub fn len(&self) -> usize {
        self.editors.len()
    }

    /// Always `false`: an area always shows an editor.
    pub fn is_empty(&self) -> bool {
        self.editors.is_empty()
    }

    /// Removes `editor`, keeping the shown editor when another one goes; `false` if absent.
    fn remove(&mut self, editor: EditorId) -> bool {
        let Some(index) = self.editors.iter().position(|e| *e == editor) else {
            return false;
        };
        self.editors.remove(index);
        if index < self.active || self.active >= self.editors.len() {
            self.active = self.active.saturating_sub(1);
        }
        true
    }

    fn insert(&mut self, index: usize, editor: EditorId) {
        let index = index.min(self.editors.len());
        self.editors.insert(index, editor);
        self.active = index;
    }
}

/// Why a workspace operation was refused. A refused operation changes nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkspaceError {
    /// The layout refused.
    Layout(LayoutError),
    /// No area shows this editor.
    UnknownEditor(EditorId),
    /// A tab index out of range.
    UnknownTab(usize),
    /// An editor alone in its area cannot dock beside itself.
    AloneInArea,
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(error) => error.fmt(f),
            Self::UnknownEditor(editor) => write!(f, "no editor {}", editor.0),
            Self::UnknownTab(index) => write!(f, "no tab {index}"),
            Self::AloneInArea => f.write_str("an editor alone in its area cannot dock beside it"),
        }
    }
}

impl std::error::Error for WorkspaceError {}

impl From<LayoutError> for WorkspaceError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

/// Result of workspace operations.
pub(crate) type Result<T> = std::result::Result<T, WorkspaceError>;

/// What a split created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Split {
    /// The new area.
    pub area: AreaId,
    /// Its editor, new.
    pub editor: EditorId,
    /// The editor the new one copies (the split area's shown editor).
    pub copy_of: EditorId,
}

/// Areas tiling the window, each holding a stack of editors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    layout: Layout,
    stacks: Vec<(AreaId, Stack)>,
    next_editor: u32,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Workspace {
    /// One area with one editor.
    pub fn new() -> Self {
        let layout = Layout::new();
        let area = layout.areas().next().map_or(AreaId(0), |(id, _)| id);
        Self {
            layout,
            stacks: vec![(area, Stack::new(EditorId(0)))],
            next_editor: 1,
        }
    }

    /// A workspace from saved data: a layout, and for each area its editors in tab order with
    /// the position of the one shown.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::Invalid`] unless every area has exactly one stack, every stack
    /// shows one of its editors, and no editor appears twice.
    pub fn from_saved(layout: Layout, stacks: &[(u32, Vec<u32>, usize)]) -> Result<Self> {
        let next_editor = stacks
            .iter()
            .flat_map(|(_, editors, _)| editors.iter().map(|id| id + 1))
            .max()
            .unwrap_or(0);
        let workspace = Self {
            layout,
            stacks: stacks
                .iter()
                .map(|(area, editors, active)| {
                    (
                        AreaId(*area),
                        Stack {
                            editors: editors.iter().map(|id| EditorId(*id)).collect(),
                            active: *active,
                        },
                    )
                })
                .collect(),
            next_editor,
        };
        workspace
            .check()
            .map_err(|reason| WorkspaceError::Layout(LayoutError::Invalid(reason)))?;
        Ok(workspace)
    }

    /// The layout.
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// The editors of `area`.
    pub fn stack(&self, area: AreaId) -> Option<&Stack> {
        self.stacks
            .iter()
            .find(|(id, _)| *id == area)
            .map(|(_, stack)| stack)
    }

    fn stack_mut(&mut self, area: AreaId) -> Result<&mut Stack> {
        self.stacks
            .iter_mut()
            .find(|(id, _)| *id == area)
            .map(|(_, stack)| stack)
            .ok_or(WorkspaceError::Layout(LayoutError::UnknownArea(area)))
    }

    /// The area showing `editor` among its tabs.
    pub fn area_of(&self, editor: EditorId) -> Option<AreaId> {
        self.stacks
            .iter()
            .find(|(_, stack)| stack.editors.contains(&editor))
            .map(|(id, _)| *id)
    }

    /// Every editor, area by area.
    pub fn editors(&self) -> impl Iterator<Item = EditorId> + '_ {
        self.stacks
            .iter()
            .flat_map(|(_, stack)| stack.editors.iter().copied())
    }

    fn new_editor(&mut self) -> EditorId {
        let editor = EditorId(self.next_editor);
        self.next_editor += 1;
        editor
    }

    /// Splits `area` (see [`Layout::split`]); the new area shows a new editor, meant to copy
    /// the split area's shown editor.
    ///
    /// # Errors
    ///
    /// The layout's.
    pub fn split(
        &mut self,
        area: AreaId,
        orientation: Orientation,
        position: u32,
        min: u32,
    ) -> Result<Split> {
        let copy_of = self
            .stack(area)
            .map(Stack::active)
            .ok_or(LayoutError::UnknownArea(area))?;
        let added = self.layout.split(area, orientation, position, min)?;
        let editor = self.new_editor();
        self.stacks.push((added, Stack::new(editor)));
        Ok(Split {
            area: added,
            editor,
            copy_of,
        })
    }

    /// Moves `line` (see [`Layout::move_line`]).
    pub fn move_line(&mut self, line: &Line, position: u32, min: u32) -> u32 {
        self.layout.move_line(line, position, min)
    }

    /// Joins `removed` into `kept` (see [`Layout::join`]) and returns the removed area's
    /// editors, whose states can be dropped.
    ///
    /// # Errors
    ///
    /// The layout's.
    pub fn join(&mut self, kept: AreaId, removed: AreaId) -> Result<Vec<EditorId>> {
        self.layout.join(kept, removed)?;
        Ok(self.take_stack(removed))
    }

    /// Closes `area` (see [`Layout::close`]) and returns its editors.
    ///
    /// # Errors
    ///
    /// The layout's.
    pub fn close(&mut self, area: AreaId) -> Result<Vec<EditorId>> {
        self.layout.close(area)?;
        Ok(self.take_stack(area))
    }

    fn take_stack(&mut self, area: AreaId) -> Vec<EditorId> {
        let index = self.stacks.iter().position(|(id, _)| *id == area);
        index.map_or_else(Vec::new, |index| self.stacks.remove(index).1.editors)
    }

    /// Exchanges the editors of two areas.
    ///
    /// # Errors
    ///
    /// [`LayoutError::UnknownArea`].
    pub fn swap(&mut self, a: AreaId, b: AreaId) -> Result<()> {
        let index = |area| {
            self.stacks
                .iter()
                .position(|(id, _)| *id == area)
                .ok_or(WorkspaceError::Layout(LayoutError::UnknownArea(area)))
        };
        let (first, second) = (index(a)?, index(b)?);
        let first_stack = self.stacks[first].1.clone();
        self.stacks[first].1 = self.stacks[second].1.clone();
        self.stacks[second].1 = first_stack;
        Ok(())
    }

    /// Shows tab `index` of `area`.
    ///
    /// # Errors
    ///
    /// [`LayoutError::UnknownArea`] or [`WorkspaceError::UnknownTab`].
    pub fn activate(&mut self, area: AreaId, index: usize) -> Result<()> {
        let stack = self.stack_mut(area)?;
        if index >= stack.editors.len() {
            return Err(WorkspaceError::UnknownTab(index));
        }
        stack.active = index;
        Ok(())
    }

    /// Moves tab `from` of `area` to position `to`; the moved editor is shown.
    ///
    /// # Errors
    ///
    /// [`LayoutError::UnknownArea`] or [`WorkspaceError::UnknownTab`].
    pub fn reorder(&mut self, area: AreaId, from: usize, to: usize) -> Result<()> {
        let stack = self.stack_mut(area)?;
        if from >= stack.editors.len() {
            return Err(WorkspaceError::UnknownTab(from));
        }
        let editor = stack.editors.remove(from);
        stack.insert(to, editor);
        Ok(())
    }

    /// Adds a new editor to the tabs of `area`, after its shown one, and shows it.
    ///
    /// # Errors
    ///
    /// [`LayoutError::UnknownArea`].
    pub fn add_tab(&mut self, area: AreaId) -> Result<EditorId> {
        self.stack_mut(area)?;
        let editor = self.new_editor();
        let stack = self.stack_mut(area)?;
        let index = stack.active + 1;
        stack.insert(index, editor);
        Ok(editor)
    }

    /// Moves `editor` into the tabs of `target` at `index`, and shows it. Its former area
    /// closes when left empty; when it cannot close, nothing moves.
    ///
    /// Returns the closed area, if any.
    ///
    /// # Errors
    ///
    /// [`WorkspaceError::UnknownEditor`], [`LayoutError::UnknownArea`], or the layout's errors
    /// when the former area cannot close.
    pub fn dock_in_tabs(
        &mut self,
        editor: EditorId,
        target: AreaId,
        index: usize,
    ) -> Result<Option<AreaId>> {
        let source = self
            .area_of(editor)
            .ok_or(WorkspaceError::UnknownEditor(editor))?;
        let mut next = self.clone();
        next.stack_mut(target)?;
        let closed = next.take_editor(editor, source)?;
        next.stack_mut(target)?.insert(index, editor);
        *self = next;
        Ok(closed)
    }

    /// Moves `editor` to the `side` of `target`, which splits in half: the editor alone in the
    /// new half. Its former area closes when left empty; when it cannot close, nothing moves.
    ///
    /// Returns the new area.
    ///
    /// # Errors
    ///
    /// [`WorkspaceError::UnknownEditor`], [`WorkspaceError::AloneInArea`] when the editor is
    /// alone in `target`, or the layout's errors (halves smaller than `min`, a former area
    /// that cannot close).
    pub fn dock_at_side(
        &mut self,
        editor: EditorId,
        target: AreaId,
        side: Side,
        min: u32,
    ) -> Result<AreaId> {
        let source = self
            .area_of(editor)
            .ok_or(WorkspaceError::UnknownEditor(editor))?;
        if source == target && self.stack(source).is_some_and(|stack| stack.len() == 1) {
            return Err(WorkspaceError::AloneInArea);
        }
        let rect = self
            .layout
            .rect(target)
            .ok_or(LayoutError::UnknownArea(target))?;
        let orientation = side.orientation();
        let (start, end) = match orientation {
            Orientation::Vertical => (rect.x0, rect.x1),
            Orientation::Horizontal => (rect.y0, rect.y1),
        };
        let middle = start + (end - start) / 2;
        let mut next = self.clone();
        // The split keeps the first half in `target`; the editor goes to the half on `side`.
        let added = next.layout.split(target, orientation, middle, min)?;
        let (editor_area, other_area) = if side.is_end() {
            (added, target)
        } else {
            (target, added)
        };
        if other_area != target {
            // The target's editors move to the second half.
            let stack = next.take_stack_entry(target)?;
            next.stacks.push((other_area, stack));
        }
        next.stacks.push((editor_area, Stack::new(editor)));
        // The editor leaves its former stack, which moved along if it was the target's.
        let former = if source == target { other_area } else { source };
        next.take_editor(editor, former)?;
        *self = next;
        Ok(editor_area)
    }

    fn take_stack_entry(&mut self, area: AreaId) -> Result<Stack> {
        let index = self
            .stacks
            .iter()
            .position(|(id, _)| *id == area)
            .ok_or(WorkspaceError::Layout(LayoutError::UnknownArea(area)))?;
        Ok(self.stacks.remove(index).1)
    }

    /// Takes `editor` out of `source`, closing `source` when left empty.
    fn take_editor(&mut self, editor: EditorId, source: AreaId) -> Result<Option<AreaId>> {
        let stack = self.stack_mut(source)?;
        stack.remove(editor);
        if !stack.is_empty() {
            return Ok(None);
        }
        self.layout.close(source)?;
        self.take_stack(source);
        Ok(Some(source))
    }

    /// Checks the layout and that each area has a non-empty stack, each editor appearing once.
    ///
    /// # Errors
    ///
    /// The first problem found, described.
    pub fn check(&self) -> std::result::Result<(), String> {
        self.layout.check()?;
        if self.stacks.len() != self.layout.len() {
            return Err(format!(
                "{} stacks for {} areas",
                self.stacks.len(),
                self.layout.len()
            ));
        }
        let mut seen = Vec::new();
        for (area, stack) in &self.stacks {
            if self.layout.rect(*area).is_none() {
                return Err(format!("stack of missing area {}", area.0));
            }
            if stack.editors.is_empty() || stack.active >= stack.editors.len() {
                return Err(format!("area {} has a broken stack", area.0));
            }
            for editor in &stack.editors {
                if seen.contains(editor) {
                    return Err(format!("editor {} appears twice", editor.0));
                }
                seen.push(*editor);
            }
        }
        Ok(())
    }
}

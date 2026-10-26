//! Workspace operations: editors in stacks, docking, and a long run of random operations.

use koebi_workspace::{
    AreaId, EditorId, GRID, GridRect, Layout, LayoutError, Orientation, Side, Workspace,
    WorkspaceError,
};

const MIN: u32 = 1_000;

fn only_area(workspace: &Workspace) -> AreaId {
    workspace
        .layout()
        .areas()
        .next()
        .expect("a layout has an area")
        .0
}

/// Left and right halves, the right one holding three editors.
fn halves_with_tabs() -> (Workspace, AreaId, AreaId, [EditorId; 3]) {
    let mut workspace = Workspace::new();
    let left = only_area(&workspace);
    let split = workspace
        .split(left, Orientation::Vertical, GRID / 2, MIN)
        .expect("the grid splits in halves");
    let right = split.area;
    let second = workspace.add_tab(right).expect("the area exists");
    let third = workspace.add_tab(right).expect("the area exists");
    (workspace, left, right, [split.editor, second, third])
}

#[test]
fn splits_create_an_editor_copying_the_shown_one() {
    let mut workspace = Workspace::new();
    let area = only_area(&workspace);
    let shown = workspace.stack(area).unwrap().active();
    let split = workspace
        .split(area, Orientation::Horizontal, GRID / 2, MIN)
        .unwrap();
    assert_eq!(split.copy_of, shown);
    assert_ne!(split.editor, shown);
    assert_eq!(workspace.area_of(split.editor), Some(split.area));
    workspace.check().unwrap();
}

#[test]
fn tabs_reorder_and_activate() {
    let (mut workspace, _, right, [a, b, c]) = halves_with_tabs();
    let stack = workspace.stack(right).unwrap();
    assert_eq!(stack.editors(), [a, b, c]);
    assert_eq!(stack.active(), c);
    workspace.reorder(right, 2, 0).unwrap();
    let stack = workspace.stack(right).unwrap();
    assert_eq!((stack.editors(), stack.active()), (&[c, a, b][..], c));
    workspace.activate(right, 2).unwrap();
    assert_eq!(workspace.stack(right).unwrap().active(), b);
    assert_eq!(
        workspace.activate(right, 3),
        Err(WorkspaceError::UnknownTab(3))
    );
    assert_eq!(
        workspace.reorder(right, 5, 0),
        Err(WorkspaceError::UnknownTab(5))
    );
    workspace.check().unwrap();
}

#[test]
fn docking_into_tabs_moves_the_editor_and_closes_an_emptied_area() {
    let (mut workspace, left, right, [a, _, _]) = halves_with_tabs();
    // One editor of the right area joins the left area's tabs: both areas stay.
    assert_eq!(workspace.dock_in_tabs(a, left, 1), Ok(None));
    assert_eq!(workspace.area_of(a), Some(left));
    assert_eq!(workspace.stack(left).unwrap().active(), a);
    assert_eq!(workspace.stack(right).unwrap().len(), 2);
    workspace.check().unwrap();
    // The left area's editors all leave: it closes, and the right area takes its space.
    let left_editors = workspace.stack(left).unwrap().editors().to_vec();
    for editor in &left_editors[..left_editors.len() - 1] {
        workspace.dock_in_tabs(*editor, right, 0).unwrap();
    }
    let last = left_editors[left_editors.len() - 1];
    assert_eq!(workspace.dock_in_tabs(last, right, 9), Ok(Some(left)));
    assert_eq!(workspace.layout().len(), 1);
    assert_eq!(workspace.layout().rect(right), Some(GridRect::FULL));
    assert_eq!(
        workspace.stack(right).unwrap().editors().last(),
        Some(&last)
    );
    workspace.check().unwrap();
}

#[test]
fn docking_at_a_side_splits_the_target_in_half() {
    let (mut workspace, left, right, [a, b, _]) = halves_with_tabs();
    // Docking above the left area: the left area's editors keep the bottom half.
    let top = workspace.dock_at_side(a, left, Side::Top, MIN).unwrap();
    assert_eq!(
        workspace.layout().rect(top),
        Some(GridRect::new(0, 0, GRID / 2, GRID / 2))
    );
    assert_eq!(workspace.stack(top).unwrap().editors(), [a]);
    let bottom = workspace
        .layout()
        .area_at(0, GRID - 1)
        .expect("an area at the bottom left");
    assert_ne!(bottom, top);
    workspace.check().unwrap();
    // Docking beside its own area splits it.
    let beside = workspace.dock_at_side(b, right, Side::Right, MIN).unwrap();
    assert_eq!(
        workspace.layout().rect(beside),
        Some(GridRect::new(3 * GRID / 4, 0, GRID, GRID))
    );
    assert_eq!(workspace.area_of(b), Some(beside));
    workspace.check().unwrap();
    // Alone in an area, an editor cannot dock beside it.
    assert_eq!(
        workspace.dock_at_side(a, top, Side::Left, MIN),
        Err(WorkspaceError::AloneInArea)
    );
}

#[test]
fn docking_is_refused_when_the_emptied_area_cannot_close() {
    // The centre of a pinwheel cannot close: its only editor cannot leave.
    let mut workspace = Workspace::new();
    let first = only_area(&workspace);
    let (a, b) = (GRID / 3, 2 * GRID / 3);
    // Build the pinwheel by splits and line moves: north arm, then the rest.
    let rest = workspace
        .split(first, Orientation::Horizontal, a, MIN)
        .unwrap()
        .area;
    let north = first;
    let east = workspace
        .split(north, Orientation::Vertical, b, MIN)
        .unwrap()
        .area;
    let below = workspace
        .split(rest, Orientation::Horizontal, b, MIN)
        .unwrap()
        .area;
    let west = workspace
        .split(rest, Orientation::Vertical, a, MIN)
        .unwrap();
    let centre = west.area;
    let west = rest;
    let east_lower = workspace
        .split(centre, Orientation::Vertical, b, MIN)
        .unwrap()
        .area;
    // East arm: join the area right of the centre into the north-east one.
    workspace.join(east, east_lower).unwrap();
    // South arm: the bottom row minus the west column; west arm: the left column.
    let south_left = workspace
        .split(below, Orientation::Vertical, a, MIN)
        .unwrap();
    workspace.join(west, below).unwrap();
    let south = south_left.area;
    assert_eq!(
        workspace.layout().rect(centre),
        Some(GridRect::new(a, a, b, b))
    );
    assert_eq!(workspace.layout().len(), 5);
    workspace.check().unwrap();

    let editor = workspace.stack(centre).unwrap().active();
    let before = workspace.clone();
    assert_eq!(
        workspace.dock_in_tabs(editor, south, 0),
        Err(WorkspaceError::Layout(LayoutError::CannotFill))
    );
    assert_eq!(
        workspace.dock_at_side(editor, south, Side::Top, MIN),
        Err(WorkspaceError::Layout(LayoutError::CannotFill))
    );
    assert_eq!(workspace, before);
}

#[test]
fn swapping_exchanges_editors_and_joining_returns_the_removed_ones() {
    let (mut workspace, left, right, [a, b, c]) = halves_with_tabs();
    let left_editor = workspace.stack(left).unwrap().active();
    workspace.swap(left, right).unwrap();
    assert_eq!(workspace.stack(left).unwrap().editors(), [a, b, c]);
    assert_eq!(workspace.stack(right).unwrap().editors(), [left_editor]);
    let removed = workspace.join(left, right).unwrap();
    assert_eq!(removed, [left_editor]);
    assert_eq!(workspace.editors().collect::<Vec<_>>(), [a, b, c]);
    workspace.check().unwrap();
    let only = only_area(&workspace);
    assert_eq!(
        workspace.close(only),
        Err(WorkspaceError::Layout(LayoutError::LastArea))
    );
}

/// A deterministic pseudo-random sequence (xorshift).
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound.max(1)).expect("a small bound");
        usize::try_from(self.next() % bound).expect("below a usize bound")
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
}

#[test]
fn random_operations_keep_the_workspace_consistent() {
    let mut noise = Noise(0x5DEE_CE66_D1CE_4E5B);
    let mut workspace = Workspace::new();
    let (mut applied, mut refused) = (0, 0);
    for step in 0..4_000 {
        let areas: Vec<AreaId> = workspace.layout().areas().map(|(id, _)| id).collect();
        let editors: Vec<EditorId> = workspace.editors().collect();
        let area = noise.pick(&areas);
        let other = noise.pick(&areas);
        let editor = noise.pick(&editors);
        let side = noise.pick(&Side::ALL);
        let position = u32::try_from(noise.below(GRID as usize + 1)).unwrap();
        let orientation = noise.pick(&[Orientation::Vertical, Orientation::Horizontal]);
        let before = workspace.clone();
        // Keep the layout between 1 and 12 areas so that every operation gets exercised.
        let kind = if areas.len() > 12 {
            2 + noise.below(6)
        } else {
            noise.below(9)
        };
        let result: Result<(), WorkspaceError> = match kind {
            0 | 1 => workspace
                .split(area, orientation, position, MIN)
                .map(|_| ()),
            2 => workspace.join(area, other).map(|_| ()),
            3 => workspace.close(area).map(|_| ()),
            4 => {
                if let Some(line) = workspace.layout().line(area, side) {
                    let landed = workspace.move_line(&line, position, MIN);
                    let (low, high) = before.layout().line_limits(&line, MIN);
                    assert!(low <= landed && landed <= high, "step {step}");
                }
                Ok(())
            }
            5 => workspace.swap(area, other),
            6 => {
                let index = noise.below(4);
                workspace.dock_in_tabs(editor, other, index).map(|_| ())
            }
            7 => workspace.dock_at_side(editor, other, side, MIN).map(|_| ()),
            _ => workspace.add_tab(area).map(|_| ()),
        };
        if let Err(error) = &result {
            assert_eq!(
                workspace, before,
                "step {step}: {error} changed the workspace"
            );
            refused += 1;
        } else {
            applied += 1;
        }
        if let Err(problem) = workspace.check() {
            panic!("step {step} (operation {kind}): {problem}\n{workspace:#?}");
        }
    }
    // Both outcomes must be common, or the test explores little.
    assert!(
        applied > 1_000 && refused > 200,
        "{applied} applied, {refused} refused"
    );
}

#[test]
fn saved_workspaces_come_back_as_they_were() {
    let (workspace, ..) = halves_with_tabs();
    let areas: Vec<(u32, GridRect)> = workspace
        .layout()
        .areas()
        .map(|(id, rect)| (id.get(), rect))
        .collect();
    let stacks: Vec<(u32, Vec<u32>, usize)> = workspace
        .layout()
        .areas()
        .map(|(id, _)| {
            let stack = workspace.stack(id).unwrap();
            let editors = stack.editors().iter().map(|editor| editor.get()).collect();
            (id.get(), editors, stack.active_index())
        })
        .collect();
    let layout = Layout::from_areas(&areas).unwrap();
    let mut restored = Workspace::from_saved(layout.clone(), &stacks).unwrap();
    assert_eq!(restored, workspace);
    // New editors never reuse a saved identifier.
    let area = workspace.layout().areas().next().unwrap().0;
    let added = restored.add_tab(area).unwrap();
    assert!(workspace.editors().all(|editor| editor != added));
    // Broken data is refused.
    let mut twice = stacks.clone();
    twice[0].1.push(stacks[1].1[0]);
    assert!(Workspace::from_saved(layout.clone(), &twice).is_err());
    let mut hidden = stacks.clone();
    hidden[0].2 = 9;
    assert!(Workspace::from_saved(layout.clone(), &hidden).is_err());
    assert!(Workspace::from_saved(layout, &stacks[..1]).is_err());
}

//! Koebi's workspace model: areas tiling the window, each showing a stack of editors.
//!
//! Pure data, no drawing ([ADR 0015]): the interface draws the areas and turns the user's
//! gestures into these operations, and shows their previews from the same plans.
//!
//! - [`Layout`]: rectangles on an integer [`GRID`] that tile it exactly; [`Line`]s of aligned
//!   edges that move together; splits, joins that hand leftovers to neighbours, closes.
//! - [`Workspace`]: a layout whose areas hold [`Stack`]s of [`EditorId`]s (tabs), with
//!   swapping, reordering and docking editors into tab bars or beside areas.
//!
//! Every operation either applies completely or changes nothing.
//!
//! ```
//! use koebi_workspace::{GRID, Orientation, Side, Workspace};
//!
//! let mut workspace = Workspace::new();
//! let first = workspace.layout().areas().next().unwrap().0;
//! // Split into a left and a right half, then move the line between them.
//! let right = workspace.split(first, Orientation::Vertical, GRID / 2, 1_000)?.area;
//! let line = workspace.layout().line(first, Side::Right).unwrap();
//! assert_eq!(workspace.move_line(&line, GRID / 3, 1_000), GRID / 3);
//! // Join them back: the left area takes the whole grid.
//! workspace.join(first, right)?;
//! assert_eq!(workspace.layout().len(), 1);
//! # Ok::<(), koebi_workspace::WorkspaceError>(())
//! ```
//!
//! [ADR 0015]: https://github.com/Kolosta/Koebi/blob/develop/docs/adr/0015-workspace-layout.md

mod grid;
mod layout;
mod plan;
mod workspace;

pub use grid::{GRID, GridRect, Orientation, Side, grid_length, to_grid, to_pixel};
pub use layout::{AreaId, Layout, LayoutError, Line};
pub use plan::{ClosePlan, Fill, JoinPlan};
pub use workspace::{EditorId, Split, Stack, Workspace, WorkspaceError};

//! Actions: what the application can do, declared once in the module of their feature with
//! their default shortcuts and the context they apply in.
//!
//! ```
//! use koebi_core::input::{Key, MouseButton};
//! use koebi_input::{ActionDef, ActionGroup, ButtonTrigger, Context, Shortcut};
//!
//! /// Shortcuts of the viewport's canvas.
//! pub const CANVAS: Context = Context::new("viewport.canvas");
//!
//! /// Frames the whole document.
//! pub static VIEW_ALL: ActionDef = ActionDef {
//!     id: "viewport.view-all",
//!     label: "Frame all",
//!     description: "Zooms and pans so that the whole document is visible.",
//!     context: CANVAS,
//!     defaults: &[Shortcut::key(Key::Home)],
//! };
//!
//! /// Pans the view while the middle button drags.
//! pub static PAN: ActionDef = ActionDef {
//!     id: "viewport.pan",
//!     label: "Pan",
//!     description: "Moves the view with the pointer.",
//!     context: CANVAS,
//!     defaults: &[Shortcut::button(MouseButton::Middle, ButtonTrigger::Press)],
//! };
//!
//! /// The viewport's actions, as the shortcut editor shows them.
//! pub static VIEWPORT: ActionGroup = ActionGroup {
//!     name: "Viewport",
//!     actions: &[&VIEW_ALL, &PAN],
//! };
//! ```

use core::fmt;

use crate::shortcut::Shortcut;

/// Where shortcuts apply: a kind of zone of the interface (the application, an area, the top
/// bar of an editor, the canvas of the viewport...). The zones under the pointer form a chain
/// of contexts, innermost first; a shortcut of an inner context wins over the same shortcut
/// of an outer one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Context(&'static str);

impl Context {
    /// The context named `name` (`feature.zone`, lowercase).
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// Its name.
    pub const fn name(self) -> &'static str {
        self.0
    }
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Context({})", self.0)
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// An action, declared once as a `static` in the module of its feature.
#[derive(Debug)]
pub struct ActionDef {
    /// Stable identifier, `feature.action` in lowercase: saved shortcuts refer to it, so it is
    /// never renamed.
    pub id: &'static str,
    /// Short label, for menus and the shortcut editor.
    pub label: &'static str,
    /// One sentence, for tooltips.
    pub description: &'static str,
    /// Where its shortcuts apply.
    pub context: Context,
    /// Its default shortcuts, possibly none.
    pub defaults: &'static [Shortcut],
}

impl PartialEq for ActionDef {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ActionDef {}

/// The actions of one feature, shown together in the shortcut editor and reset together.
#[derive(Debug)]
pub struct ActionGroup {
    /// Name of the group.
    pub name: &'static str,
    /// Its actions.
    pub actions: &'static [&'static ActionDef],
}

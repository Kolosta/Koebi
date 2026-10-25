//! Koebi's actions and shortcuts.
//!
//! The code triggers **actions**; users choose the **shortcuts** of each action. Actions are
//! declared in the module of their feature ([`ActionDef`], grouped in [`ActionGroup`]s), with
//! their default shortcuts and the [`Context`] they apply in.
//!
//! - **Shortcuts** ([`Shortcut`]): keys (press, release, click, double click, repeat), mouse
//!   buttons (press, release, click, double click, drag in a direction from a distance), the
//!   wheel, pinches and pointer moves, with modifier rules that tell left and right apart
//!   ([`ModifierRule`]). Their text form (`Ctrl+Shift+Z`, `LeftMouse drag north`) is what
//!   saved keymaps hold.
//! - **Contexts**: the application gives, with each input event, the chain of contexts under
//!   the pointer (application, area, editor, top bar...), innermost first. The innermost
//!   context with a matching shortcut wins.
//! - **Conflicts**: two actions of one context never share a shortcut; the same shortcut may
//!   serve different actions in different contexts.
//! - **Overrides**: shortcuts can be changed, restored one by one, by group or all at once;
//!   an action is modified when its shortcuts differ from its defaults.
//!
//! ```
//! use std::time::Instant;
//!
//! use koebi_core::Vec2;
//! use koebi_core::input::{Key, Modifiers};
//! use koebi_input::{
//!     ActionDef, ActionGroup, Context, InputEvent, InputSettings, Keymap, Shortcut,
//! };
//!
//! const APP: Context = Context::new("app");
//! static UNDO: ActionDef = ActionDef {
//!     id: "app.undo",
//!     label: "Undo",
//!     description: "Undoes the last change.",
//!     context: APP,
//!     defaults: &[Shortcut::key(Key::Z).primary()],
//! };
//! static EDIT: ActionGroup = ActionGroup { name: "Edit", actions: &[&UNDO] };
//!
//! let mut keymap = Keymap::new(&[&EDIT])?;
//! let primary = if cfg!(any(target_os = "macos", target_os = "ios")) {
//!     Modifiers::LEFT_GUI
//! } else {
//!     Modifiers::LEFT_CTRL
//! };
//! let event = InputEvent::Key { key: Key::Z, down: true, repeat: false, modifiers: primary };
//! let triggers = keymap.handle(event, &[APP], &InputSettings::default(), Instant::now());
//! assert_eq!(triggers.iter().next().map(|t| t.action.id), Some("app.undo"));
//! # Ok::<(), koebi_input::KeymapError>(())
//! ```

mod action;
mod gesture;
mod keymap;
mod saved;
mod shortcut;
mod text;

pub use action::{ActionDef, ActionGroup, Context};
pub use gesture::{Gesture, GestureKind, InputEvent, InputSettings};
pub use keymap::{Keymap, KeymapError, Triggered, Triggers};
pub use saved::{HEADER, LoadReport};
pub use shortcut::{
    ButtonTrigger, Drag, DragDirection, KeyTrigger, ModifierRule, ModifierState, PinchDirection,
    Shortcut, Side, Trigger, WheelDirection,
};
pub use text::ParseShortcutError;

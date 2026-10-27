//! [`MenuItem`]: the entries of a menu (actions, submenus, separators, titles) and their
//! builders.

use super::state::MAX_DEPTH;
use crate::icon::Icon;

/// The mark shown before the label of an action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MenuMark {
    /// No mark, and no room kept for one.
    #[default]
    None,
    /// A check mark while on: an option that is set or not.
    Check(bool),
    /// A dot while on: one choice among the items around it.
    Radio(bool),
}

/// A choice of a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuAction<'a> {
    /// What it does.
    pub label: &'a str,
    /// An icon before the label.
    pub icon: Option<Icon>,
    /// Its shortcut, shown at the end.
    pub shortcut: Option<&'a str>,
    /// A check or radio mark.
    pub mark: MenuMark,
    /// Disabled items show but cannot be chosen.
    pub enabled: bool,
    /// The current choice of a dropdown.
    pub selected: bool,
}

/// An item opening a submenu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Submenu<'a> {
    /// What the submenu holds.
    pub label: &'a str,
    /// An icon before the label.
    pub icon: Option<Icon>,
    /// The items of the submenu.
    pub items: &'a [MenuItem<'a>],
    /// A disabled submenu shows but does not open.
    pub enabled: bool,
}

/// An entry of a menu.
///
/// Menus nest eight levels deep at most: the root and seven levels of submenus; a submenu of
/// the eighth level shows disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem<'a> {
    /// A choice.
    Action(MenuAction<'a>),
    /// An item opening a submenu.
    Submenu(Submenu<'a>),
    /// A line between groups of items.
    Separator,
    /// A heading above a group of items.
    Title(&'a str),
}

impl<'a> MenuItem<'a> {
    /// An enabled choice.
    pub const fn action(label: &'a str) -> Self {
        Self::Action(MenuAction {
            label,
            icon: None,
            shortcut: None,
            mark: MenuMark::None,
            enabled: true,
            selected: false,
        })
    }

    /// An enabled item opening a submenu of `items` (see the depth limit on [`MenuItem`]).
    pub const fn submenu(label: &'a str, items: &'a [MenuItem<'a>]) -> Self {
        Self::Submenu(Submenu {
            label,
            icon: None,
            items,
            enabled: true,
        })
    }

    /// The same choice or submenu, showing `icon` before its label.
    #[must_use]
    pub const fn icon(self, icon: Icon) -> Self {
        match self {
            Self::Action(mut action) => {
                action.icon = Some(icon);
                Self::Action(action)
            }
            Self::Submenu(mut submenu) => {
                submenu.icon = Some(icon);
                Self::Submenu(submenu)
            }
            other => other,
        }
    }

    /// The same choice, showing `shortcut`.
    #[must_use]
    pub const fn shortcut(self, shortcut: &'a str) -> Self {
        match self {
            Self::Action(mut action) => {
                action.shortcut = Some(shortcut);
                Self::Action(action)
            }
            other => other,
        }
    }

    /// The same choice or submenu, enabled or not.
    #[must_use]
    pub const fn enabled(self, enabled: bool) -> Self {
        match self {
            Self::Action(mut action) => {
                action.enabled = enabled;
                Self::Action(action)
            }
            Self::Submenu(mut submenu) => {
                submenu.enabled = enabled;
                Self::Submenu(submenu)
            }
            other => other,
        }
    }

    /// The same choice, marked as the current one or not.
    #[must_use]
    pub const fn selected(self, selected: bool) -> Self {
        match self {
            Self::Action(mut action) => {
                action.selected = selected;
                Self::Action(action)
            }
            other => other,
        }
    }

    /// The same choice with a check mark, shown while `checked`.
    #[must_use]
    pub const fn checked(self, checked: bool) -> Self {
        self.mark(MenuMark::Check(checked))
    }

    /// The same choice with a radio mark, shown while `on`.
    #[must_use]
    pub const fn radio(self, on: bool) -> Self {
        self.mark(MenuMark::Radio(on))
    }

    const fn mark(self, mark: MenuMark) -> Self {
        match self {
            Self::Action(mut action) => {
                action.mark = mark;
                Self::Action(action)
            }
            other => other,
        }
    }

    /// `true` for items of `level` that can be highlighted: enabled choices, and enabled
    /// submenus above the deepest level.
    pub(crate) fn is_active_at(&self, level: usize) -> bool {
        match self {
            Self::Action(action) => action.enabled,
            Self::Submenu(submenu) => submenu.enabled && level + 1 < MAX_DEPTH,
            Self::Separator | Self::Title(_) => false,
        }
    }

    /// The items of the submenu this item of `level` opens, if it opens one.
    pub(crate) fn opens(&self, level: usize) -> Option<&'a [MenuItem<'a>]> {
        match self {
            Self::Submenu(submenu) if self.is_active_at(level) => Some(submenu.items),
            _ => None,
        }
    }
}

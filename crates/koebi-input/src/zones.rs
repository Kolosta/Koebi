//! [`Zones`]: how the contexts of shortcuts nest, and the conflicts that follow.
//!
//! A context is a kind of zone of the interface. The application declares where each one lies
//! ([`Within`]): at the root (the whole window), inside another zone (an editor inside the
//! workspace), or inside any zone (a text field, a list). Two shortcuts that one input triggers
//! conflict as far as their contexts meet:
//!
//! - in the same context, one input would trigger both: a **total** conflict, which the keymap
//!   refuses ([`crate::Keymap::set_bindings`]), so an editor turns the newcomer off;
//! - in nested contexts, the inner one takes the input inside it: the outer one is
//!   **shadowed** there, the inner one **shadows** it;
//! - the same key or button in another phase (a press and a release, a click and a double
//!   click, a press and a drag) are **similar**: one movement of the user triggers both, or one
//!   instead of the other.
//!
//! Contexts that never meet (an editor and another editor, two kinds of fields) never conflict.

use crate::action::{ActionDef, Context};
use crate::keymap::Keymap;
use crate::shortcut::Shortcut;

/// Where a context lies among the zones of the interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Within {
    /// At the root: the whole window.
    Root,
    /// Inside the zone of this context: an editor inside the workspace.
    Zone(Context),
    /// Inside any zone: components such as text fields and lists, found everywhere.
    AnyZone,
}

/// A context as the shortcut editor shows it, and where it lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextInfo {
    /// The context.
    pub context: Context,
    /// Its name for the user: the zone or the component.
    pub label: &'static str,
    /// Where it lies.
    pub within: Within,
}

/// How two contexts meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nesting {
    /// The same context.
    Same,
    /// The first lies inside the second.
    Inside,
    /// The first holds the second.
    Around,
    /// They never meet.
    Apart,
}

/// What a conflict is, for the shortcut it is reported on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConflictKind {
    /// Not a conflict: an action of the same context has the same shortcut, but the two
    /// never apply at once (their conditions exclude each other), so the input does what fits.
    Shared,
    /// The same input does something else in a zone this shortcut's lies in, or here: one
    /// movement triggers both, or one instead of the other.
    Similar,
    /// This shortcut takes the input from the other inside its own zone, which lies inside the
    /// other's: the other is shadowed there.
    Shadows,
    /// A zone inside this shortcut's takes the input there: this one is shadowed.
    Shadowed,
    /// The same context: one input would trigger both.
    Total,
}

impl ConflictKind {
    /// `true` for the conflicts the keymap refuses: the shortcut cannot be on.
    pub const fn is_total(self) -> bool {
        matches!(self, Self::Total)
    }

    /// `true` for conflicts; `false` for a shortcut shared by actions that never apply at
    /// once.
    pub const fn is_conflict(self) -> bool {
        !matches!(self, Self::Shared)
    }
}

/// A conflict of a shortcut with another action's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// What it is.
    pub kind: ConflictKind,
    /// The other action.
    pub other: &'static ActionDef,
    /// The other action's shortcut.
    pub shortcut: Shortcut,
}

/// The contexts of the interface and where each lies.
#[derive(Clone, Copy, Debug)]
pub struct Zones {
    infos: &'static [ContextInfo],
}

impl Zones {
    /// The contexts `infos` describe. Contexts they do not name lie at the root.
    pub const fn new(infos: &'static [ContextInfo]) -> Self {
        Self { infos }
    }

    /// Every context described, in order.
    pub fn infos(&self) -> &'static [ContextInfo] {
        self.infos
    }

    /// What is known of `context`.
    pub fn info(&self, context: Context) -> Option<&'static ContextInfo> {
        self.infos.iter().find(|info| info.context == context)
    }

    /// The name of `context` for the user: its label, or its name.
    pub fn label(&self, context: Context) -> &'static str {
        self.info(context).map_or(context.name(), |info| info.label)
    }

    /// Where `context` lies: at the root when it is not described.
    pub fn within(&self, context: Context) -> Within {
        self.info(context).map_or(Within::Root, |info| info.within)
    }

    /// The zone holding `context`, if it lies inside one.
    pub fn parent(&self, context: Context) -> Option<Context> {
        match self.within(context) {
            Within::Zone(parent) => Some(parent),
            Within::Root | Within::AnyZone => None,
        }
    }

    /// `true` when `inner` lies inside `outer`: below it through the zones, or a component
    /// inside a zone; a root context holds every other one.
    pub fn holds(&self, outer: Context, inner: Context) -> bool {
        if outer == inner {
            return false;
        }
        let (outer_within, inner_within) = (self.within(outer), self.within(inner));
        match (outer_within, inner_within) {
            // Components are inside any zone, never inside another component.
            (Within::AnyZone, _) => false,
            (_, Within::AnyZone) => true,
            (Within::Root, _) if self.root_of(inner) == Some(outer) => true,
            _ => {
                // Up from `inner` through its zones, a bounded walk in case of a cycle.
                let mut current = inner;
                for _ in 0..=self.infos.len() {
                    match self.parent(current) {
                        Some(parent) if parent == outer => return true,
                        Some(parent) => current = parent,
                        None => return false,
                    }
                }
                false
            }
        }
    }

    /// The root context above `context`: itself at the root, `None` for a component.
    fn root_of(&self, context: Context) -> Option<Context> {
        let mut current = context;
        for _ in 0..=self.infos.len() {
            match self.within(current) {
                Within::Root => return Some(current),
                Within::Zone(parent) => current = parent,
                Within::AnyZone => return None,
            }
        }
        None
    }

    /// How `a` meets `b`.
    pub fn nesting(&self, a: Context, b: Context) -> Nesting {
        if a == b {
            Nesting::Same
        } else if self.holds(b, a) {
            Nesting::Inside
        } else if self.holds(a, b) {
            Nesting::Around
        } else {
            Nesting::Apart
        }
    }

    /// What `shortcut` of an action in `context` has against `other`'s, in `other_context`:
    /// `None` when they do not conflict.
    pub fn conflict(
        &self,
        (context, shortcut): (Context, Shortcut),
        (other_context, other): (Context, Shortcut),
    ) -> Option<ConflictKind> {
        let nesting = self.nesting(context, other_context);
        if nesting == Nesting::Apart {
            return None;
        }
        if shortcut.overlaps(other) {
            return Some(match nesting {
                Nesting::Same => ConflictKind::Total,
                Nesting::Inside => ConflictKind::Shadows,
                Nesting::Around => ConflictKind::Shadowed,
                Nesting::Apart => return None,
            });
        }
        shortcut.is_similar(other).then_some(ConflictKind::Similar)
    }
}

impl Keymap {
    /// Fills `conflicts` with those `shortcut` has, as a shortcut of the action `id`, with the
    /// shortcuts of the other actions that are on, the gravest first. A shortcut that is off
    /// still learns what it would conflict with.
    pub fn conflicts_in(
        &self,
        zones: &Zones,
        id: &str,
        shortcut: Shortcut,
        conflicts: &mut Vec<Conflict>,
    ) {
        conflicts.clear();
        let Some(action) = self.action(id) else {
            return;
        };
        for (other, bindings) in self.actions() {
            if other.id == action.id {
                continue;
            }
            for binding in bindings.iter().filter(|binding| binding.enabled) {
                let found = zones.conflict(
                    (action.context, shortcut),
                    (other.context, binding.shortcut),
                );
                // One context, one input, but never both at once: shared, not a conflict.
                let found = match found {
                    Some(ConflictKind::Total) if action.excludes(other) => {
                        Some(ConflictKind::Shared)
                    }
                    found => found,
                };
                if let Some(kind) = found {
                    conflicts.push(Conflict {
                        kind,
                        other,
                        shortcut: binding.shortcut,
                    });
                }
            }
        }
        conflicts.sort_by_key(|conflict| core::cmp::Reverse(conflict.kind));
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::input::{Key, ModifierKey, MouseButton};

    use super::*;
    use crate::action::ActionGroup;
    use crate::shortcut::{Binding, ButtonTrigger, Drag, KeyTrigger, Trigger};

    const APP: Context = Context::new("app");
    const WORKSPACE: Context = Context::new("workspace");
    const VIEWPORT: Context = Context::new("viewport");
    const OUTLINER: Context = Context::new("outliner");
    const TEXT: Context = Context::new("text");
    const LIST: Context = Context::new("list");
    const MENU: Context = Context::new("menu");

    static INFOS: [ContextInfo; 7] = [
        ContextInfo {
            context: APP,
            label: "Application",
            within: Within::Root,
        },
        ContextInfo {
            context: WORKSPACE,
            label: "Workspace",
            within: Within::Zone(APP),
        },
        ContextInfo {
            context: VIEWPORT,
            label: "Viewport",
            within: Within::Zone(WORKSPACE),
        },
        ContextInfo {
            context: OUTLINER,
            label: "Outliner",
            within: Within::Zone(WORKSPACE),
        },
        ContextInfo {
            context: TEXT,
            label: "Text fields",
            within: Within::AnyZone,
        },
        ContextInfo {
            context: LIST,
            label: "Lists",
            within: Within::AnyZone,
        },
        ContextInfo {
            context: MENU,
            label: "Menus",
            within: Within::Root,
        },
    ];
    const ZONES: Zones = Zones::new(&INFOS);

    #[test]
    fn zones_nest_through_their_parents_and_components_lie_anywhere() {
        assert_eq!(ZONES.nesting(VIEWPORT, VIEWPORT), Nesting::Same);
        assert_eq!(ZONES.nesting(VIEWPORT, APP), Nesting::Inside);
        assert_eq!(ZONES.nesting(APP, VIEWPORT), Nesting::Around);
        assert_eq!(ZONES.nesting(VIEWPORT, WORKSPACE), Nesting::Inside);
        // Two editors, two components, two roots: they never meet.
        assert_eq!(ZONES.nesting(VIEWPORT, OUTLINER), Nesting::Apart);
        assert_eq!(ZONES.nesting(TEXT, LIST), Nesting::Apart);
        assert_eq!(ZONES.nesting(APP, MENU), Nesting::Apart);
        // A component lies inside any zone, the root ones too.
        assert_eq!(ZONES.nesting(TEXT, VIEWPORT), Nesting::Inside);
        assert_eq!(ZONES.nesting(MENU, LIST), Nesting::Around);
        // An unknown context lies at the root.
        let other = Context::new("other");
        assert_eq!(ZONES.within(other), Within::Root);
        assert_eq!(ZONES.label(other), "other");
        assert_eq!(ZONES.label(TEXT), "Text fields");
        assert_eq!(ZONES.parent(VIEWPORT), Some(WORKSPACE));
        assert_eq!(ZONES.parent(TEXT), None);
    }

    #[test]
    fn conflicts_follow_how_the_contexts_meet() {
        let home = Shortcut::key(Key::Home);
        assert_eq!(
            ZONES.conflict((VIEWPORT, home), (VIEWPORT, home)),
            Some(ConflictKind::Total)
        );
        // The text field inside the viewport takes Home there.
        assert_eq!(
            ZONES.conflict((TEXT, home), (VIEWPORT, home)),
            Some(ConflictKind::Shadows)
        );
        assert_eq!(
            ZONES.conflict((VIEWPORT, home), (TEXT, home)),
            Some(ConflictKind::Shadowed)
        );
        assert_eq!(ZONES.conflict((VIEWPORT, home), (OUTLINER, home)), None);
        // Another modifier: nothing.
        let ctrl_home = home.with(ModifierKey::Ctrl);
        assert_eq!(
            ZONES.conflict((VIEWPORT, ctrl_home), (VIEWPORT, home)),
            None
        );
        // The press and the release of one key, a click and a drag of one button.
        let release = Shortcut::new(Trigger::Key {
            key: Key::Home,
            when: KeyTrigger::Release,
            repeat: false,
        });
        assert_eq!(
            ZONES.conflict((VIEWPORT, release), (APP, home)),
            Some(ConflictKind::Similar)
        );
        let click = Shortcut::button(MouseButton::Left, ButtonTrigger::Click);
        let drag = Shortcut::button(MouseButton::Left, ButtonTrigger::Drag(Drag::ANY));
        assert_eq!(
            ZONES.conflict((VIEWPORT, click), (VIEWPORT, drag)),
            Some(ConflictKind::Similar)
        );
        // A key held as a modifier wins over the same shortcut without it: similar only.
        let held = Shortcut::key(Key::G).holding(Key::Space);
        let plain = Shortcut::key(Key::G);
        assert!(!held.overlaps(plain));
        assert_eq!(
            ZONES.conflict((VIEWPORT, held), (VIEWPORT, plain)),
            Some(ConflictKind::Similar)
        );
        // Severities rank: total first.
        assert!(ConflictKind::Total > ConflictKind::Shadowed);
        assert!(ConflictKind::Shadowed > ConflictKind::Similar);
        assert!(ConflictKind::Total.is_total() && !ConflictKind::Shadows.is_total());
    }

    static VIEW_ALL: ActionDef = ActionDef {
        id: "viewport.view-all",
        label: "View all",
        description: "",
        context: VIEWPORT,
        defaults: &[Shortcut::key(Key::Home)],
        repeats: false,
        when: None,
        hint: None,
    };
    static LINE_START: ActionDef = ActionDef {
        id: "text.line-start",
        label: "Line start",
        description: "",
        context: TEXT,
        defaults: &[Shortcut::key(Key::Home)],
        repeats: false,
        when: None,
        hint: None,
    };
    static FRAME: ActionDef = ActionDef {
        id: "viewport.frame",
        label: "Frame",
        description: "",
        context: VIEWPORT,
        defaults: &[],
        repeats: false,
        when: None,
        hint: None,
    };
    static GROUP: ActionGroup = ActionGroup {
        name: "Test",
        actions: &[&VIEW_ALL, &LINE_START, &FRAME],
    };

    #[test]
    fn a_keymap_tells_the_conflicts_of_a_shortcut_gravest_first() {
        let mut keymap = Keymap::new(&[&GROUP]).unwrap();
        let mut conflicts = Vec::new();
        let home = Shortcut::key(Key::Home);
        keymap.conflicts_in(&ZONES, FRAME.id, home, &mut conflicts);
        let kinds: Vec<(ConflictKind, &str)> = conflicts
            .iter()
            .map(|conflict| (conflict.kind, conflict.other.id))
            .collect();
        assert_eq!(
            kinds,
            [
                (ConflictKind::Total, VIEW_ALL.id),
                (ConflictKind::Shadowed, LINE_START.id)
            ]
        );
        // The keymap refuses it on; off, it is kept, and conflicts with nothing.
        assert!(keymap.set_bindings(FRAME.id, &[Binding::on(home)]).is_err());
        keymap
            .set_bindings(FRAME.id, &[Binding::off(home)])
            .unwrap();
        keymap.conflicts_in(&ZONES, VIEW_ALL.id, home, &mut conflicts);
        assert!(
            conflicts
                .iter()
                .all(|conflict| conflict.other.id != FRAME.id)
        );
        // An unknown action has none.
        keymap.conflicts_in(&ZONES, "nothing", home, &mut conflicts);
        assert_eq!(conflicts, []);
    }
}

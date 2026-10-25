//! The keymap engine on a realistic set of actions: contexts, gestures, conflicts, overrides
//! and saved keymaps.

use std::time::{Duration, Instant};

use koebi_core::Vec2;
use koebi_core::input::{Key, ModifierKey, Modifiers, MouseButton};
use koebi_input::{
    ActionDef, ActionGroup, ButtonTrigger, Context, Drag, DragDirection, GestureKind, InputEvent,
    InputSettings, KeyTrigger, Keymap, KeymapError, ModifierRule, ModifierState, PinchDirection,
    Shortcut, Side, Trigger, WheelDirection,
};

const APP: Context = Context::new("app");
const AREA: Context = Context::new("area");
const GUTTER: Context = Context::new("gutter");
const CANVAS: Context = Context::new("viewport.canvas");

macro_rules! action {
    ($name:ident, $id:literal, $context:expr, [$($shortcut:expr),* $(,)?]) => {
        static $name: ActionDef = ActionDef {
            id: $id,
            label: $id,
            description: "An action of the tests.",
            context: $context,
            defaults: &[$($shortcut),*],
        };
    };
}

action!(
    UNDO,
    "app.undo",
    APP,
    [Shortcut::key(Key::Z).with(ModifierKey::Ctrl)]
);
action!(
    QUIT,
    "app.quit",
    APP,
    [Shortcut::key(Key::Q).with(ModifierKey::Ctrl)]
);
action!(
    MAXIMIZE,
    "area.maximize",
    AREA,
    [Shortcut::key(Key::Space).with(ModifierKey::Ctrl)]
);
action!(
    AREA_MENU,
    "gutter.menu",
    GUTTER,
    [Shortcut::button(MouseButton::Right, ButtonTrigger::Press)]
);
action!(
    RESIZE,
    "gutter.resize",
    GUTTER,
    [Shortcut::button(
        MouseButton::Left,
        ButtonTrigger::Drag(Drag::ANY)
    )]
);
action!(
    SELECT,
    "viewport.select",
    CANVAS,
    [Shortcut::button(MouseButton::Left, ButtonTrigger::Click)]
);
action!(
    BOX_SELECT,
    "viewport.box-select",
    CANVAS,
    [Shortcut::button(
        MouseButton::Left,
        ButtonTrigger::Drag(Drag::ANY)
    )]
);
action!(
    PAN_UP,
    "viewport.pan-up",
    CANVAS,
    [Shortcut::button(
        MouseButton::Middle,
        ButtonTrigger::Drag(Drag {
            direction: DragDirection::North,
            distance: Some(20),
        })
    )]
);
action!(
    ZOOM_IN,
    "viewport.zoom-in",
    CANVAS,
    [
        Shortcut::new(Trigger::Wheel(WheelDirection::Up)),
        Shortcut::new(Trigger::Pinch(PinchDirection::In)),
    ]
);
// The same shortcut as the application's undo, in an inner context: it wins on the canvas.
action!(
    CANVAS_UNDO,
    "viewport.undo-view",
    CANVAS,
    [Shortcut::key(Key::Z).with(ModifierKey::Ctrl)]
);
action!(
    STEP,
    "viewport.step",
    CANVAS,
    [Shortcut::new(Trigger::Key {
        key: Key::Right,
        when: KeyTrigger::Press,
        repeat: true,
    })]
);
action!(
    FRAME,
    "viewport.frame",
    CANVAS,
    [Shortcut::new(Trigger::Key {
        key: Key::F,
        when: KeyTrigger::DoubleClick,
        repeat: false,
    })]
);
action!(UNBOUND, "viewport.unbound", CANVAS, []);

static APPLICATION: ActionGroup = ActionGroup {
    name: "Application",
    actions: &[&UNDO, &QUIT],
};
static AREAS: ActionGroup = ActionGroup {
    name: "Areas",
    actions: &[&MAXIMIZE, &AREA_MENU, &RESIZE],
};
static VIEWPORT: ActionGroup = ActionGroup {
    name: "Viewport",
    actions: &[
        &SELECT,
        &BOX_SELECT,
        &PAN_UP,
        &ZOOM_IN,
        &CANVAS_UNDO,
        &STEP,
        &FRAME,
        &UNBOUND,
    ],
};

/// No action triggered.
const NONE: [&str; 0] = [];

/// The identifiers of the actions triggered.
fn ids(triggers: koebi_input::Triggers) -> Vec<&'static str> {
    triggers
        .iter()
        .map(|triggered| triggered.action.id)
        .collect()
}

fn keymap() -> Keymap {
    Keymap::new(&[&APPLICATION, &AREAS, &VIEWPORT]).expect("the test keymap is valid")
}

const ON_CANVAS: &[Context] = &[CANVAS, AREA, APP];
const ON_GUTTER: &[Context] = &[GUTTER, APP];

/// Feeds events and collects the identifiers of the actions they trigger.
struct Driver {
    keymap: Keymap,
    settings: InputSettings,
    now: Instant,
}

impl Driver {
    fn new() -> Self {
        Self {
            keymap: keymap(),
            settings: InputSettings::default(),
            now: Instant::now(),
        }
    }

    fn send(&mut self, event: InputEvent, chain: &[Context]) -> Vec<&'static str> {
        self.keymap
            .handle(event, chain, &self.settings, self.now)
            .iter()
            .map(|triggered| triggered.action.id)
            .collect()
    }

    fn key(
        &mut self,
        key: Key,
        down: bool,
        modifiers: Modifiers,
        chain: &[Context],
    ) -> Vec<&'static str> {
        let event = InputEvent::Key {
            key,
            down,
            repeat: false,
            modifiers,
        };
        self.send(event, chain)
    }

    fn button(
        &mut self,
        button: MouseButton,
        down: bool,
        at: Vec2,
        chain: &[Context],
    ) -> Vec<&'static str> {
        let event = InputEvent::Button {
            button,
            down,
            clicks: 1,
            position: at,
            modifiers: Modifiers::NONE,
        };
        self.send(event, chain)
    }

    fn move_to(&mut self, at: Vec2, chain: &[Context]) -> Vec<&'static str> {
        let event = InputEvent::PointerMove {
            position: at,
            modifiers: Modifiers::NONE,
        };
        self.send(event, chain)
    }
}

#[test]
fn the_innermost_context_wins() {
    let mut driver = Driver::new();
    let undo = |driver: &mut Driver, chain| driver.key(Key::Z, true, Modifiers::LEFT_CTRL, chain);
    assert_eq!(undo(&mut driver, ON_CANVAS), ["viewport.undo-view"]);
    assert_eq!(undo(&mut driver, &[AREA, APP]), ["app.undo"]);
    assert_eq!(undo(&mut driver, &[]), Vec::<&str>::new());
    // Outer contexts still answer for shortcuts the inner ones do not have.
    assert_eq!(
        driver.key(Key::Q, true, Modifiers::RIGHT_CTRL, ON_CANVAS),
        ["app.quit"]
    );
}

#[test]
fn modifiers_must_match_exactly() {
    let mut driver = Driver::new();
    let held = Modifiers::LEFT_CTRL.union(Modifiers::LEFT_SHIFT);
    assert_eq!(driver.key(Key::Z, true, held, &[APP]), NONE);
    assert_eq!(driver.key(Key::Z, true, Modifiers::NONE, &[APP]), NONE);
}

#[test]
fn a_click_is_a_release_without_a_drag() {
    let mut driver = Driver::new();
    let at = Vec2::new(100.0, 100.0);
    assert_eq!(driver.button(MouseButton::Left, true, at, ON_CANVAS), NONE);
    // A small jitter below the drag threshold keeps it a click.
    assert_eq!(driver.move_to(at + Vec2::new(2.0, 1.0), ON_CANVAS), NONE);
    assert_eq!(
        driver.button(MouseButton::Left, false, at, ON_CANVAS),
        ["viewport.select"]
    );
}

#[test]
fn moving_past_the_threshold_drags_instead_of_clicking() {
    let mut driver = Driver::new();
    let at = Vec2::new(100.0, 100.0);
    driver.button(MouseButton::Left, true, at, ON_CANVAS);
    assert_eq!(
        driver.move_to(at + Vec2::new(10.0, 0.0), ON_CANVAS),
        ["viewport.box-select"]
    );
    // A drag starts once per press, and its release is not a click.
    assert_eq!(driver.move_to(at + Vec2::new(40.0, 0.0), ON_CANVAS), NONE);
    assert_eq!(driver.button(MouseButton::Left, false, at, ON_CANVAS), NONE);
}

#[test]
fn a_drag_keeps_the_context_of_its_press() {
    let mut driver = Driver::new();
    let at = Vec2::new(300.0, 50.0);
    driver.button(MouseButton::Left, true, at, ON_GUTTER);
    // The pointer is over the canvas by the time it moves far enough.
    let triggered = driver.move_to(at + Vec2::new(30.0, 0.0), ON_CANVAS);
    assert_eq!(triggered, ["gutter.resize"]);
}

#[test]
fn drags_check_their_direction_and_own_distance() {
    let mut driver = Driver::new();
    let at = Vec2::new(100.0, 100.0);
    driver.button(MouseButton::Middle, true, at, ON_CANVAS);
    // Past the drag threshold but not the shortcut's 20 pixels.
    assert_eq!(driver.move_to(at + Vec2::new(0.0, -10.0), ON_CANVAS), NONE);
    assert_eq!(
        driver.move_to(at + Vec2::new(0.0, -25.0), ON_CANVAS),
        ["viewport.pan-up"]
    );
    driver.button(MouseButton::Middle, false, at, ON_CANVAS);

    driver.button(MouseButton::Middle, true, at, ON_CANVAS);
    assert_eq!(driver.move_to(at + Vec2::new(0.0, 25.0), ON_CANVAS), NONE);
}

#[test]
fn drag_gestures_report_their_actual_direction_and_origin() {
    let mut keymap = keymap();
    let settings = InputSettings::default();
    let now = Instant::now();
    let at = Vec2::new(10.0, 10.0);
    let press = InputEvent::Button {
        button: MouseButton::Left,
        down: true,
        clicks: 1,
        position: at,
        modifiers: Modifiers::NONE,
    };
    keymap.handle(press, ON_CANVAS, &settings, now);
    let moved = InputEvent::PointerMove {
        position: Vec2::new(18.0, 2.0),
        modifiers: Modifiers::LEFT_SHIFT,
    };
    let triggers = keymap.handle(moved, ON_CANVAS, &settings, now);
    let triggered = triggers.iter().next().expect("a drag started");
    assert_eq!(triggered.gesture.origin, at);
    assert_eq!(triggered.gesture.position, Vec2::new(18.0, 2.0));
    // The modifiers of the press count, not those of the move.
    assert_eq!(triggered.gesture.modifiers, Modifiers::NONE);
    assert!(matches!(
        triggered.gesture.kind,
        GestureKind::Button {
            when: ButtonTrigger::Drag(Drag {
                direction: DragDirection::NorthEast,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn key_double_clicks_need_two_presses_within_the_double_click_time() {
    let mut driver = Driver::new();
    assert_eq!(driver.key(Key::F, true, Modifiers::NONE, ON_CANVAS), NONE);
    driver.key(Key::F, false, Modifiers::NONE, ON_CANVAS);
    driver.now += Duration::from_millis(200);
    assert_eq!(
        driver.key(Key::F, true, Modifiers::NONE, ON_CANVAS),
        ["viewport.frame"]
    );
    driver.key(Key::F, false, Modifiers::NONE, ON_CANVAS);
    // A third press starts over; a slow second press is not a double click.
    driver.now += Duration::from_millis(100);
    assert_eq!(driver.key(Key::F, true, Modifiers::NONE, ON_CANVAS), NONE);
    driver.key(Key::F, false, Modifiers::NONE, ON_CANVAS);
    driver.now += Duration::from_secs(2);
    assert_eq!(driver.key(Key::F, true, Modifiers::NONE, ON_CANVAS), NONE);
}

#[test]
fn repeated_presses_trigger_repeating_shortcuts_only() {
    let mut driver = Driver::new();
    let repeat = |key| InputEvent::Key {
        key,
        down: true,
        repeat: true,
        modifiers: Modifiers::LEFT_CTRL,
    };
    assert_eq!(driver.send(repeat(Key::Z), &[APP]), NONE);
    let step = InputEvent::Key {
        key: Key::Right,
        down: true,
        repeat: true,
        modifiers: Modifiers::NONE,
    };
    assert_eq!(driver.send(step, ON_CANVAS), ["viewport.step"]);
}

#[test]
fn wheel_and_pinch_steps_carry_their_amount() {
    let mut keymap = keymap();
    let settings = InputSettings::default();
    let wheel = InputEvent::Wheel {
        delta: Vec2::new(0.0, 0.5),
        position: Vec2::new(5.0, 5.0),
        modifiers: Modifiers::NONE,
    };
    let triggers = keymap.handle(wheel, ON_CANVAS, &settings, Instant::now());
    let triggered = triggers.iter().next().expect("zoomed in");
    assert_eq!(triggered.action.id, "viewport.zoom-in");
    assert!(matches!(
        triggered.gesture.kind,
        GestureKind::Wheel { direction: WheelDirection::Up, amount } if (amount - 0.5).abs() < 1e-6
    ));
    let down = InputEvent::Wheel {
        delta: Vec2::new(0.0, -1.0),
        position: Vec2::ZERO,
        modifiers: Modifiers::NONE,
    };
    assert_eq!(
        ids(keymap.handle(down, ON_CANVAS, &settings, Instant::now())),
        NONE
    );
    let pinch = InputEvent::Pinch {
        scale: 1.2,
        position: Vec2::ZERO,
        modifiers: Modifiers::NONE,
    };
    let triggers = keymap.handle(pinch, ON_CANVAS, &settings, Instant::now());
    assert_eq!(
        triggers.iter().next().map(|t| t.action.id),
        Some("viewport.zoom-in")
    );
}

#[test]
fn mouse_double_clicks_come_from_the_platform_click_count() {
    static OPEN: ActionDef = ActionDef {
        id: "list.open",
        label: "Open",
        description: "Opens the item under the pointer.",
        context: Context::new("list"),
        defaults: &[Shortcut::button(
            MouseButton::Left,
            ButtonTrigger::DoubleClick,
        )],
    };
    static LIST: ActionGroup = ActionGroup {
        name: "List",
        actions: &[&OPEN],
    };
    let mut keymap = Keymap::new(&[&LIST]).expect("valid");
    let press = |clicks| InputEvent::Button {
        button: MouseButton::Left,
        down: true,
        clicks,
        position: Vec2::ZERO,
        modifiers: Modifiers::NONE,
    };
    let chain = [Context::new("list")];
    let settings = InputSettings::default();
    assert_eq!(
        ids(keymap.handle(press(1), &chain, &settings, Instant::now())),
        NONE
    );
    let triggers = keymap.handle(press(2), &chain, &settings, Instant::now());
    assert_eq!(
        triggers.iter().next().map(|t| t.action.id),
        Some("list.open")
    );
}

#[test]
fn cancelled_and_unfocused_presses_trigger_nothing_on_release() {
    let mut driver = Driver::new();
    let at = Vec2::new(10.0, 10.0);
    driver.button(MouseButton::Left, true, at, ON_CANVAS);
    // An operation took over the input.
    driver.keymap.cancel_presses();
    // Without its press, a release is not a click.
    assert_eq!(driver.button(MouseButton::Left, false, at, ON_CANVAS), NONE);

    driver.button(MouseButton::Left, true, at, ON_CANVAS);
    driver.send(InputEvent::FocusLost, ON_CANVAS);
    assert_eq!(driver.button(MouseButton::Left, false, at, ON_CANVAS), NONE);
}

#[test]
fn the_right_button_opens_the_gutter_menu_on_press() {
    let mut driver = Driver::new();
    assert_eq!(
        driver.button(MouseButton::Right, true, Vec2::ZERO, ON_GUTTER),
        ["gutter.menu"]
    );
}

#[test]
fn defaults_must_not_conflict() {
    static FIRST: ActionDef = ActionDef {
        id: "a.first",
        label: "First",
        description: "",
        context: APP,
        defaults: &[Shortcut::key(Key::A).with(ModifierKey::Ctrl)],
    };
    static SECOND: ActionDef = ActionDef {
        id: "a.second",
        label: "Second",
        description: "",
        context: APP,
        defaults: &[Shortcut::key(Key::A).modifiers(
            ModifierRule::NONE.with(ModifierKey::Ctrl, ModifierState::Down(Side::Left)),
        )],
    };
    static GROUP: ActionGroup = ActionGroup {
        name: "A",
        actions: &[&FIRST, &SECOND],
    };
    static DUPLICATE: ActionGroup = ActionGroup {
        name: "B",
        actions: &[&FIRST, &FIRST],
    };
    assert!(matches!(
        Keymap::new(&[&GROUP]),
        Err(KeymapError::Conflict {
            action: "a.second",
            with: "a.first",
            ..
        })
    ));
    assert_eq!(
        Keymap::new(&[&DUPLICATE]).err(),
        Some(KeymapError::DuplicateAction("a.first"))
    );
}

#[test]
fn shortcuts_conflict_within_a_context_only() {
    let mut keymap = keymap();
    let taken = Shortcut::key(Key::Q).with(ModifierKey::Ctrl);
    // In the same context: refused, and nothing changes.
    assert!(matches!(
        keymap.set_shortcuts("app.undo", &[taken]),
        Err(KeymapError::Conflict {
            with: "app.quit",
            ..
        })
    ));
    assert_eq!(keymap.conflicts("app.undo", taken).expect("known"), [&QUIT]);
    assert!(!keymap.is_modified("app.undo"));
    // In another context: allowed.
    keymap
        .set_shortcuts("viewport.unbound", &[taken])
        .expect("the canvas has no Ctrl+Q");
    assert_eq!(keymap.shortcuts("viewport.unbound"), Some(&[taken][..]));
    assert!(matches!(
        keymap.set_shortcuts("nowhere", &[]),
        Err(KeymapError::UnknownAction(_))
    ));
}

#[test]
fn an_action_edited_back_to_its_defaults_is_not_modified() {
    let mut keymap = keymap();
    let redo = Shortcut::key(Key::Y).with(ModifierKey::Ctrl);
    keymap.add_shortcut("app.undo", redo).expect("free");
    assert!(keymap.is_modified("app.undo"));
    assert_eq!(
        keymap
            .modified()
            .map(|action| action.id)
            .collect::<Vec<_>>(),
        ["app.undo"]
    );
    assert!(keymap.remove_shortcut("app.undo", redo).expect("known"));
    assert!(!keymap.is_modified("app.undo"));
    // Same shortcuts in another order, or repeated: still the defaults.
    keymap
        .set_shortcuts(
            "viewport.zoom-in",
            &[
                Shortcut::new(Trigger::Pinch(PinchDirection::In)),
                Shortcut::new(Trigger::Wheel(WheelDirection::Up)),
                Shortcut::new(Trigger::Pinch(PinchDirection::In)),
            ],
        )
        .expect("its own shortcuts");
    assert!(!keymap.is_modified("viewport.zoom-in"));
    assert_eq!(keymap.modified().count(), 0);
}

#[test]
fn resets_restore_defaults_by_action_group_or_all() {
    let mut keymap = keymap();
    let none: &[Shortcut] = &[];
    keymap.set_shortcuts("app.undo", none).expect("valid");
    keymap.set_shortcuts("app.quit", none).expect("valid");
    keymap.set_shortcuts("area.maximize", none).expect("valid");
    keymap.reset("app.undo").expect("its defaults are free");
    assert!(!keymap.is_modified("app.undo"));
    keymap.reset_group("Application").expect("known group");
    assert!(!keymap.is_modified("app.quit"));
    assert!(keymap.is_modified("area.maximize"));
    keymap.reset_all();
    assert_eq!(keymap.modified().count(), 0);
    assert!(matches!(
        keymap.reset_group("Nowhere"),
        Err(KeymapError::UnknownGroup(_))
    ));
}

#[test]
fn a_reset_that_would_conflict_is_refused_whole() {
    let mut keymap = keymap();
    // Give Quit the default of Undo, then try to restore Undo.
    let none: &[Shortcut] = &[];
    keymap.set_shortcuts("app.undo", none).expect("valid");
    keymap
        .set_shortcuts("app.quit", &[Shortcut::key(Key::Z).with(ModifierKey::Ctrl)])
        .expect("free now");
    assert!(matches!(
        keymap.reset("app.undo"),
        Err(KeymapError::Conflict {
            with: "app.quit",
            ..
        })
    ));
    assert_eq!(keymap.shortcuts("app.undo"), Some(none));
    // Restoring the whole group fixes both.
    keymap
        .reset_group("Application")
        .expect("both restored together");
    assert_eq!(keymap.modified().count(), 0);
}

#[test]
fn overrides_survive_saving_and_loading() {
    let mut keymap = keymap();
    keymap
        .set_shortcuts(
            "app.undo",
            &[
                Shortcut::key(Key::Z).with(ModifierKey::Ctrl),
                "LCtrl+Backspace".parse().expect("valid"),
            ],
        )
        .expect("free");
    keymap.set_shortcuts("viewport.step", &[]).expect("valid");
    let text = keymap.overrides_text();
    assert_eq!(
        text,
        "# Koebi shortcuts 1\napp.undo = Ctrl+Z, LCtrl+Backspace\nviewport.step =\n"
    );

    let mut restored = self::keymap();
    let report = restored.load_overrides(&text);
    assert!(report.is_complete(), "{report:?}");
    assert_eq!(restored.shortcuts("app.undo"), keymap.shortcuts("app.undo"));
    assert_eq!(restored.shortcuts("viewport.step"), Some(&[][..]));
    assert_eq!(restored.modified().count(), 2);
}

#[test]
fn loading_skips_and_reports_what_it_cannot_apply() {
    let mut keymap = keymap();
    let text = "# Koebi shortcuts 1\n\
                app.quit = Ctrl+Z\n\
                app.undo = Ctrl+Z\n\
                app.removed = Ctrl+R\n\
                viewport.step = NotAKey\n\
                garbage line\n\
                # a comment\n\
                area.maximize = Ctrl+M\n";
    let report = keymap.load_overrides(text);
    // Undo comes later and loses: it keeps its default, Ctrl+Z, which Quit's override still
    // uses, so Quit loses too. Both end up with their defaults, and both are reported.
    assert!(!keymap.is_modified("app.quit") && !keymap.is_modified("app.undo"));
    for id in ["app.undo:", "app.quit:"] {
        assert!(
            report.skipped.iter().any(|line| line.starts_with(id)),
            "{report:?}"
        );
    }
    assert!(
        report
            .skipped
            .iter()
            .any(|line| line.contains("unknown action app.removed"))
    );
    assert!(report.skipped.iter().any(|line| line.contains("NotAKey")));
    assert!(report.skipped.iter().any(|line| line.contains("no `=`")));
    assert_eq!(
        keymap.shortcuts("area.maximize"),
        Some(&["Ctrl+M".parse().unwrap()][..])
    );
    // Whatever the report, the keymap has no conflict.
    for group in keymap.groups() {
        for action in group.actions {
            for shortcut in keymap.shortcuts(action.id).unwrap() {
                assert_eq!(
                    keymap.conflicts(action.id, *shortcut).unwrap(),
                    Vec::<&ActionDef>::new()
                );
            }
        }
    }

    let report = keymap.load_overrides("not a keymap");
    assert!(!report.is_complete());
}

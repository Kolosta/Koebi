//! One test per bug found in review, each failing before its fix.

use koebi_core::input::Modifiers;
use koebi_core::{Rect, Vec2};
use koebi_input::Context;

use super::*;

const THIRD: WidgetId = WidgetId::new("third");

fn third() -> Rect {
    Rect::from_origin_size(Vec2::new(10.0, 70.0), Vec2::new(160.0, 22.0))
}

#[test]
fn typing_never_reaches_the_shortcuts_under_the_pointer() {
    // Regression test: keys the text context does not bind (X) went on to the context under
    // the pointer (the outliner deleted objects while a name was typed).
    const OUTLINER: Context = Context::new("outliner");
    let area = Rect::from_origin_size(Vec2::ZERO, Vec2::new(300.0, 200.0));
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    let show = |harness: &mut Harness, text: &mut String| {
        harness.frame(|ui| {
            ui.zone(area, OUTLINER, |ui| {
                TextField::new(NAME, text).show(ui, field())
            })
        })
    };
    show(&mut harness, &mut text);
    let mut chain = Vec::new();
    harness
        .state
        .chain_at(Some(Vec2::new(250.0, 150.0)), &mut chain);
    assert_eq!(chain, [OUTLINER]);
    harness.click(field().center());
    show(&mut harness, &mut text);
    harness
        .state
        .chain_at(Some(Vec2::new(250.0, 150.0)), &mut chain);
    assert_eq!(chain, [TEXT]);
}

#[test]
fn text_typed_with_tab_stays_in_the_field_that_ends() {
    // Regression test: the next field, started by Tab in the same frame, took the typed text
    // too and lost its own.
    let mut harness = Harness::new();
    let (mut first, mut second) = (String::from("Cube"), String::from("Sphere"));
    let mut both = |harness: &mut Harness| {
        harness
            .frame(|ui| {
                (
                    TextField::new(NAME, &mut first).show(ui, field()),
                    TextField::new(OTHER, &mut second).show(ui, other()),
                )
            })
            .0
    };
    both(&mut harness);
    harness.click(field().center());
    both(&mut harness);
    harness.input.typed("x");
    harness.input.trigger(&TEXT_NEXT_FIELD);
    let (_, next) = both(&mut harness);
    assert!(next.started);
    assert_eq!((first.as_str(), second.as_str()), ("x", "Sphere"));
}

#[test]
fn a_paste_the_filter_rejects_keeps_the_selection() {
    // Regression test: pasting nothing the filter accepted deleted the selection.
    let mut harness = Harness::new();
    let mut text = String::from("2026");
    let show = |harness: &mut Harness, text: &mut String| {
        harness
            .frame(|ui| {
                TextField::new(NAME, text)
                    .filter(|c| c.is_ascii_digit())
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut text);
    harness.click(field().center());
    show(&mut harness, &mut text);
    harness.state.set_pasted_text(String::from("abc"));
    let pasted = show(&mut harness, &mut text);
    assert!(!pasted.changed);
    assert_eq!(text, "2026");
}

#[test]
fn keys_and_typed_text_apply_in_the_order_they_came() {
    // Regression test: keys applied before all the text typed in the same frame, so a
    // Backspace typed after "ab" deleted an older character.
    let mut harness = Harness::new();
    let mut text = String::from("xy");
    let show = |harness: &mut Harness, text: &mut String| {
        harness
            .frame(|ui| {
                TextField::new(NAME, text)
                    .select_on_focus(SelectOnFocus::End)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut text);
    harness.click(field().center());
    show(&mut harness, &mut text);
    harness.input.typed("ab");
    harness.input.trigger(&TEXT_DELETE_BACKWARD);
    harness.input.trigger(&TEXT_LEFT);
    harness.input.typed("z");
    show(&mut harness, &mut text);
    assert_eq!(text, "xyza");
}

#[test]
fn tab_skips_disabled_fields_and_requests_to_edit_expire() {
    // Regression test: Tab landed on a disabled field and the edit just stopped; the request
    // then started the field frames later, once enabled.
    let mut harness = Harness::new();
    let mut texts = [
        String::from("Cube"),
        String::from("Locked"),
        String::from("Sphere"),
    ];
    let mut all = |harness: &mut Harness, middle: bool| {
        harness
            .frame(|ui| {
                let [first, second, last] = &mut texts;
                [
                    TextField::new(NAME, first).show(ui, field()),
                    TextField::new(OTHER, second)
                        .enabled(middle)
                        .show(ui, other()),
                    TextField::new(THIRD, last).show(ui, third()),
                ]
            })
            .0
    };
    all(&mut harness, false);
    harness.click(field().center());
    all(&mut harness, false);
    harness.input.trigger(&TEXT_NEXT_FIELD);
    let [_, locked, last] = all(&mut harness, false);
    assert!(!locked.started && last.started && last.editing);
    // A request to edit the disabled field is dropped, and does not wait for it.
    harness.frame(|ui| ui.edit_text_field(OTHER));
    all(&mut harness, false);
    all(&mut harness, false);
    let [_, enabled, _] = all(&mut harness, true);
    assert!(!enabled.started);
}

#[test]
fn asking_the_editing_field_to_edit_does_not_restart_it_later() {
    // Regression test: the request waited, and started the field again right after Enter
    // came in the same frame.
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    harness.start(&mut text);
    harness.input.trigger(&TEXT_CONFIRM);
    let (confirmed, _) = harness.frame(|ui| {
        ui.edit_text_field(NAME);
        TextField::new(NAME, &mut text).show(ui, field())
    });
    assert!(confirmed.confirmed());
    let after = harness.show(&mut text);
    assert!(!after.started && !after.editing);
}

/// One frame of two fields, the first cancelling on outside presses, drawn first or second.
fn two_fields(
    harness: &mut Harness,
    first: &mut String,
    second: &mut String,
    editing_first: bool,
) -> (TextFieldResponse, TextFieldResponse) {
    harness
        .frame(|ui| {
            let mut one = |ui: &mut Ui<'_>| {
                TextField::new(NAME, first)
                    .outside_press(OutsidePress::Cancel)
                    .show(ui, field())
            };
            if editing_first {
                let one = one(ui);
                (one, TextField::new(OTHER, second).show(ui, other()))
            } else {
                let two = TextField::new(OTHER, second).show(ui, other());
                (one(ui), two)
            }
        })
        .0
}

#[test]
fn a_press_on_another_field_cancels_as_asked_whatever_the_draw_order() {
    // Regression test: drawn after the pressed field, a field cancelling on outside presses
    // kept its text instead.
    for editing_first in [true, false] {
        let mut harness = Harness::new();
        let (mut first, mut second) = (String::from("Cube"), String::from("Sphere"));
        two_fields(&mut harness, &mut first, &mut second, editing_first);
        harness.click(field().center());
        two_fields(&mut harness, &mut first, &mut second, editing_first);
        harness.input.typed("Plane");
        two_fields(&mut harness, &mut first, &mut second, editing_first);
        assert_eq!(first, "Plane");
        harness.click(other().center());
        let (one, two) = two_fields(&mut harness, &mut first, &mut second, editing_first);
        assert!(two.started, "editing first: {editing_first}");
        let one = if one.ended.is_some() {
            one
        } else {
            // Drawn after the press ended it: it hears in the same frame or the next one.
            two_fields(&mut harness, &mut first, &mut second, editing_first).0
        };
        assert!(one.cancelled(), "editing first: {editing_first}");
        assert_eq!(first, "Cube", "editing first: {editing_first}");
    }
}

#[test]
fn presses_count_where_they_happened() {
    // Regression test: the caret went where the pointer was at the frame, not where the
    // press was; without a pointer (a touch lifted), the press selected everything.
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    let show = |harness: &mut Harness, text: &mut String| {
        harness
            .frame(|ui| {
                TextField::new(NAME, text)
                    .select_on_focus(SelectOnFocus::Pointer)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut text);
    let end = Vec2::new(field().max.x - 4.0, field().center().y);
    harness.click(end);
    harness.input.pointer_left();
    show(&mut harness, &mut text);
    harness.input.typed("X");
    show(&mut harness, &mut text);
    assert_eq!(text, "CubeX");
}

#[test]
fn shift_presses_extend_and_drags_from_a_select_all_press_select_from_it() {
    let mut harness = Harness::new();
    let mut text = String::from("snap to grid");
    let (x, _) = harness.frame(|ui| ui.text_width("snap ", &ui.styles().text.body));
    let padding = harness.styles.text_field.padding;
    let after_space = Vec2::new(field().min.x + padding + x + 1.0, field().center().y);
    let end = Vec2::new(field().max.x - 4.0, field().center().y);
    // A press that starts the edit, then a drag past the threshold: from the press.
    harness.show(&mut text);
    harness.input.primary_button(true, after_space);
    harness.show(&mut text);
    harness.input.pointer_moved(end);
    harness.show(&mut text);
    harness.input.primary_button(false, end);
    harness.show(&mut text);
    harness.input.typed("x");
    harness.show(&mut text);
    assert_eq!(text, "snap x");
    // Shift and a press: the selection extends from the caret to the press.
    harness.input.set_modifiers(Modifiers::LEFT_SHIFT);
    harness.click(after_space);
    harness.show(&mut text);
    harness.input.set_modifiers(Modifiers::NONE);
    harness.input.typed("y");
    harness.show(&mut text);
    assert_eq!(text, "snap y");
}

#[test]
fn length_limits_count_the_selection_as_replaced_and_lone_fields_keep_tab() {
    let mut harness = Harness::new();
    let mut text = String::from("2026");
    let show = |harness: &mut Harness, text: &mut String| {
        harness
            .frame(|ui| TextField::new(NAME, text).max_chars(4).show(ui, field()))
            .0
    };
    show(&mut harness, &mut text);
    harness.click(field().center());
    show(&mut harness, &mut text);
    harness.input.typed("19999");
    show(&mut harness, &mut text);
    assert_eq!(text, "1999");
    // Tab with no other field ends the edit, and nothing starts again.
    harness.input.trigger(&TEXT_NEXT_FIELD);
    assert!(show(&mut harness, &mut text).confirmed());
    let after = show(&mut harness, &mut text);
    assert!(!after.started && !after.editing);
}

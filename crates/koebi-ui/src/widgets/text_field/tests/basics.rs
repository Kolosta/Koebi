//! Starting and ending edits, the pointer, the keys, the clipboard, Tab, constraints,
//! scrolling and the caret.

use core::time::Duration;

use koebi_core::{Rect, Vec2};

use super::*;

#[test]
fn a_press_starts_editing_with_the_text_selected() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    let idle = harness.show(&mut text);
    assert!(!idle.editing && !idle.started);
    let started = harness.start(&mut text);
    assert!(started.started && started.editing);
    // While it edits, the field takes the keyboard and asks for text input over itself.
    let mut chain = Vec::new();
    harness.state.chain_at(None, &mut chain);
    assert_eq!(chain, [TEXT]);
    assert_eq!(harness.state.text_input_area(), Some(field()));
    assert!(harness.state.is_editing_text());
    // Typing replaces the selection, and the text changes as it is typed.
    harness.input.typed("Sph\u{e8}re");
    let typed = harness.show(&mut text);
    assert!(typed.changed && typed.response.changed && typed.editing);
    assert_eq!(text, "Sph\u{e8}re");
}

#[test]
fn enter_keeps_the_text_and_escape_restores_it() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    harness.start(&mut text);
    harness.input.typed("Plane");
    harness.show(&mut text);
    harness.input.trigger(&TEXT_CONFIRM);
    let confirmed = harness.show(&mut text);
    assert!(confirmed.confirmed() && !confirmed.editing);
    assert_eq!(text, "Plane");
    assert!(!harness.state.is_editing_text());

    harness.start(&mut text);
    harness.input.typed("Torus");
    harness.show(&mut text);
    assert_eq!(text, "Torus");
    harness.input.trigger(&TEXT_CANCEL);
    let cancelled = harness.show(&mut text);
    assert!(cancelled.cancelled() && cancelled.changed && !cancelled.editing);
    assert_eq!(text, "Plane");
}

#[test]
fn a_press_outside_keeps_the_text_or_restores_it() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    harness.start(&mut text);
    harness.input.typed("Plane");
    harness.show(&mut text);
    harness.click(Vec2::new(300.0, 200.0));
    let outside = harness.show(&mut text);
    assert!(outside.confirmed() && !outside.editing);
    assert_eq!(text, "Plane");

    let show = |harness: &mut Harness, text: &mut String| {
        harness
            .frame(|ui| {
                TextField::new(NAME, text)
                    .outside_press(OutsidePress::Cancel)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut text);
    harness.click(field().center());
    show(&mut harness, &mut text);
    harness.input.typed("Torus");
    show(&mut harness, &mut text);
    harness.click(Vec2::new(300.0, 200.0));
    let cancelled = show(&mut harness, &mut text);
    assert!(cancelled.cancelled());
    assert_eq!(text, "Plane");
}

#[test]
fn keys_move_the_caret_select_and_delete() {
    let mut harness = Harness::new();
    let mut text = String::from("snap to grid");
    harness.start(&mut text);
    for action in [&TEXT_END, &TEXT_WORD_LEFT, &TEXT_DELETE_WORD_BACKWARD] {
        harness.input.trigger(action);
    }
    harness.show(&mut text);
    assert_eq!(text, "snap grid");
    harness.input.trigger(&TEXT_START);
    harness.input.trigger(&TEXT_SELECT_WORD_RIGHT);
    harness.show(&mut text);
    harness.input.typed("Snap");
    harness.show(&mut text);
    assert_eq!(text, "Snap grid");
    harness.input.trigger(&TEXT_SELECT_END);
    harness.input.trigger(&TEXT_DELETE_BACKWARD);
    harness.show(&mut text);
    assert_eq!(text, "Snap");
    // Undo and redo step through the edits.
    harness.input.trigger(&TEXT_UNDO);
    harness.show(&mut text);
    assert_eq!(text, "Snap grid");
    harness.input.trigger(&TEXT_REDO);
    harness.show(&mut text);
    assert_eq!(text, "Snap");
}

#[test]
fn the_clipboard_goes_through_the_application() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    harness.start(&mut text);
    harness.input.trigger(&TEXT_COPY);
    harness.show(&mut text);
    assert_eq!(harness.state.copied_text(), Some("Cube"));
    harness.input.trigger(&TEXT_CUT);
    harness.show(&mut text);
    assert_eq!(
        (harness.state.copied_text(), text.as_str()),
        (Some("Cube"), "")
    );
    // The next frame no longer asks.
    harness.show(&mut text);
    assert_eq!(harness.state.copied_text(), None);
    harness.input.trigger(&TEXT_PASTE);
    harness.show(&mut text);
    assert!(harness.state.paste_requested());
    // The application answers between frames: pasted text is one line.
    harness.state.set_pasted_text(String::from("Cube\n.001"));
    let pasted = harness.show(&mut text);
    assert!(pasted.changed && !harness.state.paste_requested());
    assert_eq!(text, "Cube .001");
    // Pasting is one undo step.
    harness.input.trigger(&TEXT_UNDO);
    harness.show(&mut text);
    assert_eq!(text, "");
}

#[test]
fn tab_keeps_the_text_and_edits_the_next_field() {
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
    harness.input.trigger(&TEXT_NEXT_FIELD);
    let (left, next) = both(&mut harness);
    assert!(left.confirmed());
    assert!(next.started && next.editing, "{next:?}");
    // Shift+Tab goes back. The previous field was drawn before this one: it starts on the
    // next frame, which the key asks for.
    harness.input.trigger(&TEXT_PREVIOUS_FIELD);
    let (_, left) = both(&mut harness);
    assert!(left.confirmed());
    assert!(harness.state.redraw_at().is_some());
    let (back, _) = both(&mut harness);
    assert!(back.started && back.editing);
    // From the first field, Shift+Tab wraps around to the last one.
    harness.input.trigger(&TEXT_PREVIOUS_FIELD);
    let (_, wrapped) = both(&mut harness);
    assert!(wrapped.started);
}

#[test]
fn another_field_starting_ends_the_edit_and_says_so() {
    let mut harness = Harness::new();
    let (mut first, mut second) = (String::from("Cube"), String::from("Sphere"));
    // The second field is drawn first: its press ends the first field's edit before the
    // first field draws.
    let mut both = |harness: &mut Harness| {
        harness
            .frame(|ui| {
                let second = TextField::new(OTHER, &mut second).show(ui, other());
                (TextField::new(NAME, &mut first).show(ui, field()), second)
            })
            .0
    };
    both(&mut harness);
    harness.click(field().center());
    both(&mut harness);
    harness.click(other().center());
    let (ended, started) = both(&mut harness);
    assert!(ended.confirmed() && !ended.editing);
    assert!(started.started && started.editing);
}

#[test]
fn a_field_no_longer_shown_ends_its_edit() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    harness.start(&mut text);
    harness.input.typed("Plane");
    harness.show(&mut text);
    // A frame without the field (its panel closed).
    harness.frame(|_| ());
    assert!(!harness.state.is_editing_text());
    assert_eq!(harness.state.text_input_area(), None);
    let back = harness.show(&mut text);
    assert!(back.confirmed() && !back.editing);
    assert_eq!(text, "Plane");
}

#[test]
fn filters_and_limits_keep_the_text_valid() {
    let mut harness = Harness::new();
    let mut text = String::new();
    let show = |harness: &mut Harness, text: &mut String| {
        harness
            .frame(|ui| {
                TextField::new(NAME, text)
                    .filter(|c| c.is_ascii_digit())
                    .max_chars(3)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut text);
    harness.click(field().center());
    show(&mut harness, &mut text);
    harness.input.typed("a1b2c3d4");
    show(&mut harness, &mut text);
    assert_eq!(text, "123");
    harness.input.trigger(&TEXT_PASTE);
    show(&mut harness, &mut text);
    harness.state.set_pasted_text(String::from("456"));
    show(&mut harness, &mut text);
    assert_eq!(text, "123");
}

#[test]
fn presses_place_the_caret_double_presses_select_words_and_drags_select() {
    let mut harness = Harness::new();
    let mut text = String::from("snap to grid");
    harness.start(&mut text);
    // Past the end of the text: the caret goes to the end; a second press selects the word.
    let end = Vec2::new(field().max.x - 4.0, field().center().y);
    harness.input.set_time(Duration::from_secs(1));
    harness.click(end);
    harness.show(&mut text);
    harness.input.set_time(Duration::from_millis(1100));
    harness.input.primary_button(true, end);
    let double = harness.show(&mut text);
    assert!(double.response.double_clicked);
    harness.input.primary_button(false, end);
    harness.show(&mut text);
    harness.input.typed("mesh");
    harness.show(&mut text);
    assert_eq!(text, "snap to mesh");
    // A drag from the start to the end selects everything.
    harness.input.set_time(Duration::from_secs(3));
    let start = Vec2::new(field().min.x + 7.0, field().center().y);
    harness.input.primary_button(true, start);
    harness.show(&mut text);
    harness.input.pointer_moved(end);
    harness.show(&mut text);
    harness.input.primary_button(false, end);
    harness.show(&mut text);
    harness.input.typed("x");
    harness.show(&mut text);
    assert_eq!(text, "x");
}

#[test]
fn long_text_scrolls_to_keep_the_caret_in_view() {
    let mut harness = Harness::new();
    let mut text = "a long name that does not fit in the field at all".to_owned();
    let narrow = Rect::from_origin_size(Vec2::new(10.0, 10.0), Vec2::new(60.0, 22.0));
    let show = |harness: &mut Harness, text: &mut String| {
        harness
            .frame(|ui| {
                TextField::new(NAME, text)
                    .select_on_focus(SelectOnFocus::End)
                    .show(ui, narrow)
            })
            .0
    };
    show(&mut harness, &mut text);
    harness.click(narrow.center());
    show(&mut harness, &mut text);
    let scroll = |harness: &Harness| harness.state.text.session.as_ref().map(|s| s.scroll);
    assert!(scroll(&harness).is_some_and(|scroll| scroll > 100.0));
    harness.input.trigger(&TEXT_START);
    show(&mut harness, &mut text);
    assert_eq!(scroll(&harness), Some(0.0));
}

#[test]
fn the_field_draws_its_selection_and_caret_while_editing() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    let ((), idle) = harness.frame(|ui| {
        TextField::new(NAME, &mut text).show(ui, field());
    });
    // The box and the text.
    assert_eq!(idle.rect_count(), 1);
    assert_eq!(idle.glyph_count(), 4);
    harness.click(field().center());
    let ((), selected) = harness.frame(|ui| {
        TextField::new(NAME, &mut text).show(ui, field());
    });
    // The box, the selection and the caret.
    assert_eq!(selected.rect_count(), 3);
    harness.input.trigger(&TEXT_END);
    let ((), caret) = harness.frame(|ui| {
        TextField::new(NAME, &mut text).show(ui, field());
    });
    assert_eq!(caret.rect_count(), 2);
    // An empty field shows its placeholder.
    let mut empty = String::new();
    let ((), placeholder) = harness.frame(|ui| {
        TextField::new(OTHER, &mut empty)
            .placeholder("Search")
            .icon(Icon::Search)
            .show(ui, other());
    });
    assert_eq!(placeholder.glyph_count(), 6);
    assert!(placeholder.line_count() > 0);
}

#[test]
fn the_caret_blinks_only_when_its_token_says_so() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    harness.start(&mut text);
    harness.input.trigger(&TEXT_END);
    harness.show(&mut text);
    // Steady by default: nothing asks for another frame.
    harness.show(&mut text);
    assert_eq!(harness.state.redraw_at(), None);
    harness.styles.text_field.caret_blink = Duration::from_millis(500);
    harness.input.set_time(Duration::from_millis(100));
    harness.input.trigger(&TEXT_LEFT);
    harness.show(&mut text);
    harness.input.set_time(Duration::from_millis(200));
    let ((), shown) = harness.frame(|ui| {
        TextField::new(NAME, &mut text).show(ui, field());
    });
    assert_eq!(harness.state.redraw_at(), Some(Duration::from_millis(600)));
    harness.input.set_time(Duration::from_millis(700));
    let ((), hidden) = harness.frame(|ui| {
        TextField::new(NAME, &mut text).show(ui, field());
    });
    assert_eq!(hidden.rect_count() + 1, shown.rect_count());
    assert_eq!(harness.state.redraw_at(), Some(Duration::from_millis(1100)));
}

#[test]
fn changes_from_the_application_during_an_edit_are_kept() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    harness.start(&mut text);
    text.replace_range(.., "Cube.001");
    harness.show(&mut text);
    harness.input.typed("2");
    harness.show(&mut text);
    assert_eq!(text, "Cube.0012");
    harness.input.trigger(&TEXT_UNDO);
    harness.show(&mut text);
    assert_eq!(text, "Cube.001");
}

#[test]
fn disabled_fields_do_not_edit_and_stop_editing() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    let show = |harness: &mut Harness, text: &mut String, enabled: bool| {
        harness
            .frame(|ui| {
                TextField::new(NAME, text)
                    .enabled(enabled)
                    .show(ui, field())
            })
            .0
    };
    show(&mut harness, &mut text, false);
    harness.click(field().center());
    assert!(!show(&mut harness, &mut text, false).editing);
    harness.click(field().center());
    assert!(show(&mut harness, &mut text, true).editing);
    let stopped = show(&mut harness, &mut text, false);
    assert!(stopped.confirmed() && !stopped.editing);
}

#[test]
fn the_application_can_start_an_edit() {
    let mut harness = Harness::new();
    let mut text = String::from("Cube");
    harness.show(&mut text);
    harness.frame(|ui| ui.edit_text_field(NAME));
    let started = harness.show(&mut text);
    assert!(started.started && started.editing);
    harness.input.typed("Plane");
    harness.show(&mut text);
    assert_eq!(text, "Plane");
}

#[test]
fn carets_and_hit_tests_follow_the_glyphs() {
    let mut harness = Harness::new();
    let ((first, end, middle), _) = harness.frame(|ui| {
        let font = ui.styles().text.body.clone();
        ui.lay_out("ab", &font);
        let line = &ui.state.line;
        let b = line.glyphs()[1].x;
        (
            caret_x(line, 1),
            caret_x(line, 2),
            (offset_at(line, b - 1.0, 2), offset_at(line, b + 1.0, 2)),
        )
    });
    assert!(first > 0.0 && end > first);
    assert_eq!(middle, (1, 1));
    assert_eq!(scroll_to(0.0, 50.0, 40.0, 60.0), 0.0);
    assert_eq!(scroll_to(0.0, 150.0, 200.0, 60.0), 90.0);
    assert_eq!(scroll_to(90.0, 20.0, 200.0, 60.0), 20.0);
}

#[test]
fn repeated_keys_between_two_frames_all_apply() {
    // Regression test: consuming the first press of a key hid its repeats in the same frame.
    let mut harness = Harness::new();
    let mut text = String::from("snap to grid");
    harness.start(&mut text);
    harness.input.trigger(&TEXT_END);
    for _ in 0..4 {
        harness.input.trigger(&TEXT_SELECT_LEFT);
    }
    harness.show(&mut text);
    harness.input.trigger(&TEXT_CUT);
    harness.show(&mut text);
    assert_eq!(harness.state.copied_text(), Some("grid"));
    assert_eq!(text, "snap to ");
}

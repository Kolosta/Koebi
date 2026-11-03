//! The pixel grid: what must line up or stay centered does so on whole physical pixels, at
//! every scale factor (fold arrows and the tree lines below them, panel chevrons, icons in
//! buttons too small for their paddings); the space around a header's controls and between
//! panels is the same number of pixels wherever they lie; a bar's fill stays inside its
//! field's rounded corners.

mod common;

use std::ops::Range;

use common::{over, render, render_scaled};
use koebi_core::{Color, Extent, Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_render::{RectPrimitive, Scene};
use koebi_testkit::gpu_or_skip;
use koebi_text::Fonts;
use koebi_tokens::KOEBI;
use koebi_ui::{
    BarMapping, Button, CollapsiblePanel, CollapsiblePanelState, Icon, Input, ListItem,
    ListOptions, ListState, NumberField, Styles, Tone, Ui, UiState, WidgetId,
};

/// The size of the sheet of icon buttons.
const SHEET: Extent = Extent::new(320, 240);

/// The scale factors the alignments hold at.
const SCALES: [f32; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn design() -> DesignSystem {
    DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Vec2::new(x, y), Vec2::new(width, height))
}

/// A pixel coordinate inside the image.
fn px(value: f32) -> u32 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive pixel positions"
    )]
    let pixel = value as u32;
    pixel
}

/// `logical` in physical pixels at `scale`, rounded.
fn physical(logical: f32, scale: f32) -> u32 {
    px((logical * scale).round())
}

/// The horizontal middle, in physical pixels, of what differs from `fill` in the pixels
/// `columns` and `rows`, each pixel weighing by how much it differs.
fn ink_middle(
    image: &koebi_testkit::Image,
    columns: Range<u32>,
    rows: Range<u32>,
    fill: [u8; 4],
) -> f64 {
    let (mut weight, mut moment) = (0.0, 0.0);
    for y in rows {
        for x in columns.clone() {
            let pixel = image.pixel(x, y);
            let ink: u32 = (0..3).map(|c| u32::from(pixel[c].abs_diff(fill[c]))).sum();
            weight += f64::from(ink);
            moment += f64::from(ink) * (f64::from(x) + 0.5);
        }
    }
    assert!(weight > 0.0, "nothing drawn there");
    moment / weight
}

#[test]
fn a_fold_arrow_points_along_the_line_of_its_children_at_every_scale() {
    let Some(gpu) =
        gpu_or_skip("a_fold_arrow_points_along_the_line_of_its_children_at_every_scale")
    else {
        return;
    };
    let styles = Styles::resolve(&design());
    let items = [
        ListItem::new(1, ""),
        ListItem::new(2, "").with_depth(1),
        ListItem::new(3, "").with_depth(1),
    ];
    let options = ListOptions {
        zebra: false,
        ..ListOptions::TREE
    };
    let fill = styles.background.to_rgba8();
    for scale in SCALES {
        let mut scene = Scene::new();
        scene.set_clear_color(styles.background);
        let (mut state, input) = (UiState::new(), Input::new());
        state.set_scale_factor(scale);
        let mut list = ListState::new();
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        let bounds = rect(0.0, 0.0, 100.0, 80.0);
        ui.list(
            WidgetId::new("tree"),
            bounds,
            &mut list,
            &items,
            &options,
            &mut (),
        );
        drop(ui);
        let px = |logical: f32| physical(logical, scale);
        let image = render_scaled(&gpu, Extent::new(px(100.0), px(80.0)), &scene, scale);
        // The arrow's slot: 16 pixels after 4 of padding, in the first row, 20 pixels high;
        // below it, the line along its two children.
        let columns = px(4.0)..px(20.0);
        let arrow = ink_middle(&image, columns.clone(), 0..px(20.0), fill);
        let line = ink_middle(&image, columns, px(20.0)..px(60.0), fill);
        assert!(
            (arrow - line).abs() < 0.05,
            "at {scale}: the arrow's middle is at {arrow}, the line's at {line}"
        );
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn a_panel_chevron_lies_on_the_center_of_a_pixel_at_every_scale() {
    let Some(gpu) = gpu_or_skip("a_panel_chevron_lies_on_the_center_of_a_pixel_at_every_scale")
    else {
        return;
    };
    let styles = Styles::resolve(&design());
    let style = styles.collapsible_panel;
    let fill = style.header_background.to_rgba8();
    let area = rect(10.0, 10.0, 120.0, 60.0);
    for scale in SCALES {
        let mut scene = Scene::new();
        scene.set_clear_color(styles.background);
        let (mut state, input) = (UiState::new(), Input::new());
        state.set_scale_factor(scale);
        let mut panel = CollapsiblePanelState::new(true);
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        CollapsiblePanel::new(WidgetId::new("panel"), &mut panel, "").show(
            &mut ui,
            area,
            |_, _| 20.0,
        );
        drop(ui);
        let size = Extent::new(physical(140.0, scale), physical(80.0, scale));
        let image = render_scaled(&gpu, size, &scene, scale);
        // The chevron's slot, at the start of the header; open, it points down, symmetric
        // around the vertical axis through its middle.
        let start = area.min.x + style.header_padding;
        let middle = start + style.chevron_size * 0.5;
        let end = start + style.chevron_size;
        let columns = physical(start - 1.0, scale)..physical(end + 1.0, scale);
        // Rows clear of the frame's border, which crosses the whole header.
        let bottom = area.min.y + style.header_height - 3.0;
        let rows = physical(area.min.y + 3.0, scale)..physical(bottom, scale);
        let shown = ink_middle(&image, columns, rows, fill);
        let expected = f64::from((middle * scale).floor()) + 0.5;
        assert!(
            (shown - expected).abs() < 0.05,
            "at {scale}: the chevron's middle is at {shown}, the pixel's at {expected}"
        );
    }
    gpu.assert_no_validation_errors();
}

/// The first and last columns of `cell` where something other than `fill` shows, in the rows
/// clear of its rounded corners (`radius`).
fn inked_columns(
    image: &koebi_testkit::Image,
    cell: Rect,
    radius: f32,
    fill: [u8; 4],
) -> Option<(u32, u32)> {
    let (top, bottom) = (px(cell.min.y + radius), px(cell.max.y - radius));
    let inked = |x: u32| (top..bottom).any(|y| image.pixel(x, y) != fill);
    let columns = px(cell.min.x)..px(cell.max.x);
    let first = columns.clone().find(|&x| inked(x))?;
    let last = columns.rev().find(|&x| inked(x))?;
    Some((first, last))
}

#[test]
fn an_icon_is_centered_in_a_button_too_narrow_for_its_paddings() {
    let Some(gpu) = gpu_or_skip("an_icon_is_centered_in_a_button_too_narrow_for_its_paddings")
    else {
        return;
    };
    let design = design();
    let styles = Styles::resolve(&design);
    let icons = [Icon::Reset, Icon::Check, Icon::Minus, Icon::Close];
    let sizes = [16.0, 18.0, 20.0, 24.0, 40.0];
    let cell = |row: u64, column: u64| {
        let side = sizes[usize::try_from(column).expect("five sizes")];
        let at = |index: u64| 8.0 + 48.0 * f32::from(u8::try_from(index).expect("a few cells"));
        rect(at(column), at(row), side, side)
    };
    let mut scene = Scene::new();
    scene.set_clear_color(styles.background);
    let (mut state, input) = (UiState::new(), Input::new());
    state.set_fonts(Fonts::with_embedded());
    let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
    for (row, &icon) in (0_u64..).zip(&icons) {
        for (column, _) in (0_u64..).zip(&sizes) {
            Button::new(WidgetId::new("icon").with(row * 8 + column))
                .icon(icon)
                .show(&mut ui, cell(row, column));
        }
    }
    drop(ui);
    let image = render(&gpu, SHEET, &scene);
    let fill = over(
        styles.button.colors(Tone::DEFAULT).background.normal,
        styles.background,
    )
    .to_rgba8();
    for (row, icon) in (0_u64..).zip(&icons) {
        for (column, _) in (0_u64..).zip(&sizes) {
            let cell = cell(row, column);
            let (first, last) = inked_columns(&image, cell, styles.button.corner_radius, fill)
                .unwrap_or_else(|| panic!("{icon:?} in {cell:?} shows nothing"));
            // As much room on each side, to the pixel, and none of the icon cut away.
            let (left, right) = (first - px(cell.min.x), px(cell.max.x) - 1 - last);
            assert!(
                left.abs_diff(right) <= 1,
                "{icon:?} in {cell:?}: {left} pixels on its left, {right} on its right"
            );
            assert!(
                left > 0 && right > 0,
                "{icon:?} in {cell:?} touches its sides"
            );
        }
    }
    gpu.assert_no_validation_errors();
}

/// The rows of column `x` whose color is not `background`, as runs: (first row, end row).
fn runs(image: &koebi_testkit::Image, x: u32, background: [u8; 4]) -> Vec<(u32, u32)> {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for y in 0..image.height() {
        if image.pixel(x, y) == background {
            continue;
        }
        match runs.last_mut() {
            Some(run) if run.1 == y => run.1 = y + 1,
            _ => runs.push((y, y + 1)),
        }
    }
    runs
}

#[test]
fn header_controls_and_stacked_panels_keep_the_same_space_wherever_they_lie() {
    let Some(gpu) =
        gpu_or_skip("header_controls_and_stacked_panels_keep_the_same_space_wherever_they_lie")
    else {
        return;
    };
    let design = design();
    let mark = Color::from_hex(0xFF_00_FF);
    for scale in SCALES {
        let styles = Styles::resolve_at(&design, scale);
        let background = styles.background.to_rgba8();
        let mut seen = None;
        // Offsets that put the panels' edges at many fractions of a physical pixel.
        for step in 0..9_u8 {
            let top = 10.0 + f32::from(step) * 0.137;
            let mut scene = Scene::new();
            scene.set_clear_color(styles.background);
            let (mut state, input) = (UiState::new(), Input::new());
            state.set_scale_factor(scale);
            let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
            let mut y = top;
            for index in 0..2_u64 {
                let mut panel = CollapsiblePanelState::new(false);
                let id = WidgetId::new("panel").with(index);
                let response = CollapsiblePanel::new(id, &mut panel, "")
                    .trailing(30.0)
                    .show(&mut ui, rect(10.0, y, 120.0, 200.0), |_, _| 0.0);
                let control = response.trailing.expect("room for a control");
                ui.scene().push_rect(&RectPrimitive::new(control, mark));
                y += response.height + styles.collapsible_panel.gap;
            }
            drop(ui);
            let image = render_scaled(
                &gpu,
                Extent::new(physical(140.0, scale), physical(100.0, scale)),
                &scene,
                scale,
            );
            // Down the middle of the controls: the panels, and the control inside each.
            let x = physical(115.0, scale);
            let panels = runs(&image, x, background);
            assert_eq!(panels.len(), 2, "at {scale}, from {top}: {panels:?}");
            let mut spaces = Vec::new();
            for panel in &panels {
                let marked: Vec<u32> = (panel.0..panel.1)
                    .filter(|&y| image.pixel(x, y) == mark.to_rgba8())
                    .collect();
                let (first, end) = (marked[0], marked[marked.len() - 1] + 1);
                assert_eq!(marked.len(), (end - first) as usize, "one run of control");
                spaces.push((first - panel.0, panel.1 - end));
            }
            let measured = (spaces[0], panels[1].0 - panels[0].1);
            // As much above a control as below; the same in both panels and wherever they lie.
            assert_eq!(spaces[0].0, spaces[0].1, "at {scale}, from {top}");
            assert_eq!(spaces[0], spaces[1], "at {scale}, from {top}");
            assert_eq!(
                *seen.get_or_insert(measured),
                measured,
                "at {scale}, from {top}"
            );
        }
    }
    gpu.assert_no_validation_errors();
}

#[test]
fn a_bar_s_fill_stays_inside_the_field_s_rounded_corners_near_either_end() {
    let Some(gpu) =
        gpu_or_skip("a_bar_s_fill_stays_inside_the_field_s_rounded_corners_near_either_end")
    else {
        return;
    };
    let styles = Styles::resolve(&design());
    let field = |index: u8| rect(10.0, 10.0 + 40.0 * f32::from(index), 200.0, 24.0);
    // Nearly empty and nearly full, the fill's end within the corners' radius; then empty.
    let sheet = |values: [f64; 2]| {
        let mut scene = Scene::new();
        scene.set_clear_color(styles.background);
        let (mut state, input) = (UiState::new(), Input::new());
        state.set_fonts(Fonts::with_embedded());
        let mut ui = Ui::new(&mut scene, &styles, &input, &mut state);
        for (index, mut value) in (0_u8..).zip(values) {
            NumberField::new(WidgetId::new("bar").with(u64::from(index)), &mut value)
                .range(0.0, 1.0)
                .bar(BarMapping::Linear)
                .corner_radius(10.0)
                .show(&mut ui, field(index));
        }
        drop(ui);
        render(&gpu, Extent::new(240, 90), &scene)
    };
    let filled = sheet([0.01, 0.995]);
    let empty = sheet([0.0, 0.0]);
    let border = styles.number_field.border_width;
    for (index, x) in [
        (0, field(0).min.x + border),
        (1, field(1).max.x - border - 2.0),
    ] {
        // A pixel in the corner, the fill reaching it but outside the rounded shape: as if
        // there were no fill.
        let (x, y) = (px(x), px(field(index).min.y + border));
        let (shown, expected) = (filled.pixel(x, y), empty.pixel(x, y));
        let difference = (0..4).map(|c| shown[c].abs_diff(expected[c])).max();
        assert!(
            difference <= Some(2),
            "the fill shows in the corner of field {index}: {shown:?} instead of {expected:?}"
        );
    }
    gpu.assert_no_validation_errors();
}

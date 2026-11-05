//! The Dev page, in development builds: what helps develop the application. For now, the
//! frame-time overlay (frames per second, CPU and GPU time of a frame), which F12 shows or hides
//! as well.

use koebi_core::{Rect, Vec2};
use koebi_design::DesignSystem;
use koebi_tokens::semantic;
use koebi_ui::{Checkbox, Ui, WidgetId};

use super::Stack;

const ID: WidgetId = WidgetId::new("settings-dev");

/// Shows the page below what `stack` holds: its options in a container that never folds.
pub(super) fn show(
    ui: &mut Ui<'_>,
    stack: &mut Stack,
    design: &DesignSystem,
    frame_times: &mut bool,
) {
    let row = design.get(semantic::SIZE_CONTROL_HEIGHT);
    let gap = ui.styles().property_group.row_gap;
    // The section's title, then the option on two lines: its label and what it does.
    let height = row + gap + 2.0 * row;
    stack.container(ui, height, |ui, content| {
        let title = Rect::from_origin_size(content.min, Vec2::new(content.width(), row));
        ui.section_title(title, "Diagnostics");
        let option = Rect::from_origin_size(
            Vec2::new(content.min.x, title.max.y + gap),
            Vec2::new(content.width(), 2.0 * row),
        );
        Checkbox::new(ID.with(1), frame_times)
            .label("Show the frame times")
            .description(
                "Frames per second, CPU and GPU time of a frame, above the status bar. F12 too.",
            )
            .show(ui, option);
    });
}

#[cfg(test)]
mod tests {
    use super::super::Page;
    use super::super::tests::Harness;

    #[test]
    fn the_option_shows_and_hides_the_frame_times() {
        let mut harness = Harness::new();
        let at = harness.page_button(Page::Dev);
        harness.click(at);
        harness.settle();
        assert_eq!(harness.settings.page(), Page::Dev);
        assert!(!harness.frame_times);
        // The checkbox, below the section's title.
        let checkbox = harness.dev_option();
        harness.click(checkbox);
        assert!(harness.frame_times);
        harness.click(checkbox);
        assert!(!harness.frame_times);
    }
}

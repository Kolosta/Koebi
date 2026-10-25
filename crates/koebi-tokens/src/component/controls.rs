//! Controls: buttons, icons, toggles, checkboxes, sliders, value fields, dropdowns, frames,
//! separators, scrollbars, tabs, headers, color swatches and theme cards.

use crate::semantic;
use koebi_design::define::*;

koebi_design::tokens! {
    group CONTROLS: Component, start = semantic::TEXT::END;

    /// Button: background color.
    BUTTON_BACKGROUND_COLOR_DEFAULT = "button.background.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Button: background color, hovered.
    BUTTON_BACKGROUND_COLOR_HOVER = "button.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Button: background color, pressed.
    BUTTON_BACKGROUND_COLOR_PRESSED = "button.background.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;
    /// Button: label color.
    BUTTON_LABEL_COLOR_DEFAULT = "button.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Button: border color.
    BUTTON_BORDER_COLOR_DEFAULT = "button.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Button: border width.
    BUTTON_BORDER_WIDTH_DEFAULT = "button.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_NONE;
    /// Button: corner radius.
    BUTTON_CORNER_RADIUS_DEFAULT = "button.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;

    /// Icon button: background color.
    ICON_BUTTON_BACKGROUND_COLOR_DEFAULT = "icon-button.background.color.default": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Icon button: background color, hovered.
    ICON_BUTTON_BACKGROUND_COLOR_HOVER = "icon-button.background.color.hover": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Icon button: background color, pressed.
    ICON_BUTTON_BACKGROUND_COLOR_PRESSED = "icon-button.background.color.pressed": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Icon button: border color.
    ICON_BUTTON_BORDER_COLOR_DEFAULT = "icon-button.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Icon button: icon color.
    ICON_BUTTON_ICON_COLOR_DEFAULT = "icon-button.icon.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Icon button: icon color negative.
    ICON_BUTTON_ICON_COLOR_NEGATIVE = "icon-button.icon.color.negative": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Icon button: corner radius.
    ICON_BUTTON_CORNER_RADIUS_DEFAULT = "icon-button.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;

    /// Icon: size of line icons.
    ICON_SIZE_DEFAULT = "icon.size.default": Dimension = px(16.0), range(8.0, 64.0);
    /// Icon: stroke width of line icons (check marks excepted).
    ICON_STROKE_WIDTH_DEFAULT = "icon.stroke-width.default": Dimension = px(1.5), range(0.5, 4.0);

    /// Toggle: background color.
    TOGGLE_BACKGROUND_COLOR_DEFAULT = "toggle.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Toggle: background color, hovered.
    TOGGLE_BACKGROUND_COLOR_HOVER = "toggle.background.color.hover": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Toggle: background color, selected.
    TOGGLE_BACKGROUND_COLOR_SELECTED = "toggle.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Toggle: border color.
    TOGGLE_BORDER_COLOR_DEFAULT = "toggle.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Toggle: border color, selected.
    TOGGLE_BORDER_COLOR_SELECTED = "toggle.border.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Toggle: label color.
    TOGGLE_LABEL_COLOR_DEFAULT = "toggle.label.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Toggle: label color, selected.
    TOGGLE_LABEL_COLOR_SELECTED = "toggle.label.color.selected": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Toggle: corner radius.
    TOGGLE_CORNER_RADIUS_DEFAULT = "toggle.corner-radius.default": Dimension -> semantic::RADIUS_S;
    /// Toggle: border width.
    TOGGLE_BORDER_WIDTH_DEFAULT = "toggle.border.width.default": Dimension =
        px(1.0), range(0.0, 4.0);

    /// Checkbox: mark color.
    CHECKBOX_MARK_COLOR_DEFAULT = "checkbox.mark.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Checkbox: background color, selected.
    CHECKBOX_BACKGROUND_COLOR_SELECTED = "checkbox.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Checkbox: background color.
    CHECKBOX_BACKGROUND_COLOR_DEFAULT = "checkbox.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Checkbox: background color, hovered.
    CHECKBOX_BACKGROUND_COLOR_HOVER = "checkbox.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Checkbox: background color, pressed.
    CHECKBOX_BACKGROUND_COLOR_PRESSED = "checkbox.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CHILD;
    /// Checkbox: background color, selected and hovered.
    CHECKBOX_BACKGROUND_COLOR_SELECTED_HOVER = "checkbox.background.color.selected-hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Checkbox: background color, selected and pressed.
    CHECKBOX_BACKGROUND_COLOR_SELECTED_PRESSED =
        "checkbox.background.color.selected-pressed": Color -> semantic::BACKGROUND_ACCENT_PRESSED;
    /// Checkbox: border color.
    CHECKBOX_BORDER_COLOR_DEFAULT = "checkbox.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Checkbox: border color, selected.
    CHECKBOX_BORDER_COLOR_SELECTED = "checkbox.border.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Checkbox: box size.
    CHECKBOX_BOX_SIZE_DEFAULT = "checkbox.box-size.default": Dimension = px(16.0), range(0.0, 40.0);
    /// Checkbox: corner radius.
    CHECKBOX_CORNER_RADIUS_DEFAULT = "checkbox.corner-radius.default": Dimension ->
        semantic::RADIUS_S;
    /// Checkbox: border width.
    CHECKBOX_BORDER_WIDTH_DEFAULT = "checkbox.border.width.default": Dimension =
        px(1.0), range(0.0, 4.0);
    /// Checkbox: stroke width of the check mark.
    CHECKBOX_MARK_WIDTH_DEFAULT = "checkbox.mark.width.default": Dimension =
        px(2.0), range(0.5, 6.0);

    /// Slider: grab color.
    SLIDER_GRAB_COLOR_DEFAULT = "slider.grab.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Slider: grab color, pressed.
    SLIDER_GRAB_COLOR_PRESSED = "slider.grab.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;
    /// Slider: corner radius.
    SLIDER_CORNER_RADIUS_DEFAULT = "slider.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Slider: grab color, hovered.
    SLIDER_GRAB_COLOR_HOVER = "slider.grab.color.hover": Color -> semantic::BACKGROUND_ACCENT_HOVER;
    /// Slider: grab width.
    SLIDER_GRAB_WIDTH_DEFAULT = "slider.grab.width.default": Dimension =
        px(10.0), range(2.0, 40.0);
    /// Slider: space between the grab and the track edges.
    SLIDER_GRAB_PADDING_DEFAULT = "slider.grab.padding.default": Dimension =
        px(2.0), range(0.0, 10.0);
    /// Slider: track color.
    SLIDER_TRACK_COLOR_DEFAULT = "slider.track.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Slider: track color, hovered.
    SLIDER_TRACK_COLOR_HOVER = "slider.track.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;

    /// Drag value: background color.
    DRAG_VALUE_BACKGROUND_COLOR_DEFAULT = "drag-value.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Drag value: background color, hovered.
    DRAG_VALUE_BACKGROUND_COLOR_HOVER = "drag-value.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Drag value: background color, pressed.
    DRAG_VALUE_BACKGROUND_COLOR_PRESSED = "drag-value.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Drag value: background color, while dragging.
    DRAG_VALUE_BACKGROUND_COLOR_DRAG = "drag-value.background.color.drag": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Drag value: text color.
    DRAG_VALUE_TEXT_COLOR_DEFAULT = "drag-value.text.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Drag value: unit color.
    DRAG_VALUE_UNIT_COLOR_DEFAULT = "drag-value.unit.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Drag value: step button color.
    DRAG_VALUE_STEP_BUTTON_COLOR_DEFAULT = "drag-value.step-button.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Drag value: step button background color, hovered.
    DRAG_VALUE_STEP_BUTTON_BACKGROUND_COLOR_HOVER =
        "drag-value.step-button.background.color.hover": Color -> semantic::BACKGROUND_APP_CONTROL;
    /// Drag value: border color.
    DRAG_VALUE_BORDER_COLOR_DEFAULT = "drag-value.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Drag value: corner radius.
    DRAG_VALUE_CORNER_RADIUS_DEFAULT = "drag-value.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Drag value: border width.
    DRAG_VALUE_BORDER_WIDTH_DEFAULT = "drag-value.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Drag value: fill showing where the value lies in its range.
    DRAG_VALUE_FILL_COLOR_DEFAULT = "drag-value.fill.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;

    /// Dropdown: height.
    DROPDOWN_SIZE_HEIGHT = "dropdown.size.height": Dimension -> semantic::SIZE_CONTROL_HEIGHT;
    /// Dropdown: horizontal padding.
    DROPDOWN_PADDING_HORIZONTAL_DEFAULT = "dropdown.padding.horizontal.default": Dimension =
        px(8.0);
    /// Dropdown: vertical padding.
    DROPDOWN_PADDING_VERTICAL_DEFAULT = "dropdown.padding.vertical.default": Dimension = px(2.0);
    /// Dropdown: chevron size.
    DROPDOWN_CHEVRON_SIZE = "dropdown.chevron.size": Dimension = px(8.0), range(0.0, 32.0);
    /// Dropdown: icon size.
    DROPDOWN_ICON_SIZE = "dropdown.icon.size": Dimension = px(16.0), range(0.0, 64.0);
    /// Dropdown: background color.
    DROPDOWN_BACKGROUND_COLOR_DEFAULT = "dropdown.background.color.default": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Dropdown: background color, hovered.
    DROPDOWN_BACKGROUND_COLOR_HOVER = "dropdown.background.color.hover": Color ->
        semantic::BACKGROUND_SURFACE_CANVAS;
    /// Dropdown: background color, pressed.
    DROPDOWN_BACKGROUND_COLOR_PRESSED = "dropdown.background.color.pressed": Color ->
        semantic::BACKGROUND_SURFACE_RAISED;
    /// Dropdown: background color, open.
    DROPDOWN_BACKGROUND_COLOR_OPEN = "dropdown.background.color.open": Color ->
        semantic::BACKGROUND_APP_FRAME_HOVER;
    /// Dropdown: background color, hovered (minimal style).
    DROPDOWN_BACKGROUND_COLOR_HOVER_MINIMAL = "dropdown.background.color.hover-minimal": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Dropdown: label color.
    DROPDOWN_LABEL_COLOR_DEFAULT = "dropdown.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Dropdown: icon color.
    DROPDOWN_ICON_COLOR_DEFAULT = "dropdown.icon.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Dropdown: corner radius.
    DROPDOWN_CORNER_RADIUS_DEFAULT = "dropdown.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Dropdown: border color.
    DROPDOWN_BORDER_COLOR_DEFAULT = "dropdown.border.color.default": Color ->
        semantic::NEUTRAL_COLOR_700;
    /// Dropdown: border color, hovered.
    DROPDOWN_BORDER_COLOR_HOVER = "dropdown.border.color.hover": Color ->
        semantic::NEUTRAL_COLOR_600;
    /// Dropdown: border width.
    DROPDOWN_BORDER_WIDTH_DEFAULT = "dropdown.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;

    /// Combo: background color.
    COMBO_BACKGROUND_COLOR_DEFAULT = "combo.background.color.default": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Combo: background color, hovered.
    COMBO_BACKGROUND_COLOR_HOVER = "combo.background.color.hover": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Combo: background color, pressed.
    COMBO_BACKGROUND_COLOR_PRESSED = "combo.background.color.pressed": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Combo: preview label color.
    COMBO_PREVIEW_LABEL_COLOR_DEFAULT = "combo.preview.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Combo: arrow icon color.
    COMBO_ARROW_ICON_COLOR_DEFAULT = "combo.arrow.icon.color.default": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Combo: popup background color.
    COMBO_POPUP_BACKGROUND_COLOR_DEFAULT = "combo.popup.background.color.default": Color ->
        semantic::BACKGROUND_FLOATING_DEFAULT;
    /// Combo: item background color, hovered.
    COMBO_ITEM_BACKGROUND_COLOR_HOVER = "combo.item.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Combo: item background color, selected.
    COMBO_ITEM_BACKGROUND_COLOR_SELECTED = "combo.item.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Combo: border color.
    COMBO_BORDER_COLOR_DEFAULT = "combo.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Combo: corner radius.
    COMBO_CORNER_RADIUS_DEFAULT = "combo.corner-radius.default": Dimension -> semantic::RADIUS_M;
    /// Combo: border width.
    COMBO_BORDER_WIDTH_DEFAULT = "combo.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Combo: horizontal padding.
    COMBO_PADDING_HORIZONTAL_DEFAULT = "combo.padding.horizontal.default": Dimension = px(4.0);
    /// Combo: vertical padding.
    COMBO_PADDING_VERTICAL_DEFAULT = "combo.padding.vertical.default": Dimension = px(4.0);

    /// Frame: background color.
    FRAME_BACKGROUND_COLOR_DEFAULT = "frame.background.color.default": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Frame: background color, hovered.
    FRAME_BACKGROUND_COLOR_HOVER = "frame.background.color.hover": Color ->
        semantic::BACKGROUND_APP_FRAME;
    /// Frame: background color, pressed.
    FRAME_BACKGROUND_COLOR_PRESSED = "frame.background.color.pressed": Color ->
        semantic::BACKGROUND_APP_CHILD;
    /// Frame: corner radius.
    FRAME_CORNER_RADIUS_DEFAULT = "frame.corner-radius.default": Dimension ->
        semantic::RADIUS_CONTROL;
    /// Frame: border width.
    FRAME_BORDER_WIDTH_DEFAULT = "frame.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Frame: horizontal padding.
    FRAME_PADDING_HORIZONTAL_DEFAULT = "frame.padding.horizontal.default": Dimension = px(4.0);
    /// Frame: vertical padding.
    FRAME_PADDING_VERTICAL_DEFAULT = "frame.padding.vertical.default": Dimension = px(4.0);
    /// Frame: input text cursor color.
    FRAME_INPUT_TEXT_CURSOR_COLOR_DEFAULT = "frame.input-text.cursor.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;

    /// Capture field: background color.
    CAPTURE_FIELD_BACKGROUND_COLOR_DEFAULT = "capture-field.background.color.default": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Capture field: background color, while recording a shortcut.
    CAPTURE_FIELD_BACKGROUND_COLOR_RECORDING = "capture-field.background.color.recording": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Capture field: background color, hovered.
    CAPTURE_FIELD_BACKGROUND_COLOR_HOVER = "capture-field.background.color.hover": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Capture field: background color, pressed.
    CAPTURE_FIELD_BACKGROUND_COLOR_PRESSED = "capture-field.background.color.pressed": Color ->
        semantic::BACKGROUND_RAISED_DEFAULT;
    /// Capture field: border color.
    CAPTURE_FIELD_BORDER_COLOR_DEFAULT = "capture-field.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Capture field: border color, while recording a shortcut.
    CAPTURE_FIELD_BORDER_COLOR_RECORDING = "capture-field.border.color.recording": Color ->
        semantic::BACKGROUND_NEGATIVE_DEFAULT;
    /// Capture field: label color.
    CAPTURE_FIELD_LABEL_COLOR_DEFAULT = "capture-field.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Capture field: label color, hint.
    CAPTURE_FIELD_LABEL_COLOR_HINT = "capture-field.label.color.hint": Color ->
        semantic::TEXT_COLOR_SECONDARY_DEFAULT;
    /// Capture field: corner radius.
    CAPTURE_FIELD_CORNER_RADIUS_DEFAULT = "capture-field.corner-radius.default": Dimension ->
        semantic::RADIUS_XS;
    /// Capture field: horizontal padding.
    CAPTURE_FIELD_PADDING_HORIZONTAL_DEFAULT =
        "capture-field.padding.horizontal.default": Dimension = px(8.0);
    /// Capture field: vertical padding.
    CAPTURE_FIELD_PADDING_VERTICAL_DEFAULT = "capture-field.padding.vertical.default": Dimension =
        px(4.0);
    /// Capture field: minimum width.
    CAPTURE_FIELD_SIZE_MIN_WIDTH = "capture-field.size.min-width": Dimension =
        px(180.0), range(80.0, 600.0);
    /// Capture field: height.
    CAPTURE_FIELD_SIZE_HEIGHT = "capture-field.size.height": Dimension =
        px(28.0), range(20.0, 64.0);

    /// Key cap: background color.
    KEY_CAP_BACKGROUND_COLOR_DEFAULT = "key-cap.background.color.default": Color ->
        semantic::BACKGROUND_APP_CONTROL;
    /// Key cap: border color.
    KEY_CAP_BORDER_COLOR_DEFAULT = "key-cap.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Key cap: label color.
    KEY_CAP_LABEL_COLOR_DEFAULT = "key-cap.label.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;
    /// Key cap: corner radius.
    KEY_CAP_CORNER_RADIUS_DEFAULT = "key-cap.corner-radius.default": Dimension ->
        semantic::RADIUS_XS;
    /// Key cap: horizontal padding.
    KEY_CAP_PADDING_HORIZONTAL_DEFAULT = "key-cap.padding.horizontal.default": Dimension = px(4.0);
    /// Key cap: vertical padding.
    KEY_CAP_PADDING_VERTICAL_DEFAULT = "key-cap.padding.vertical.default": Dimension = px(4.0);
    /// Key cap: font scale.
    KEY_CAP_FONT_SCALE_DEFAULT = "key-cap.font-scale.default": Number = 0.85, range(0.5, 2.0);

    /// Separator: color.
    SEPARATOR_COLOR_DEFAULT = "separator.color.default": Color -> semantic::BORDER_COLOR_DEFAULT;
    /// Separator: color, hovered.
    SEPARATOR_COLOR_HOVER = "separator.color.hover": Color -> semantic::BACKGROUND_ACCENT_PRESSED;
    /// Separator: color, pressed.
    SEPARATOR_COLOR_PRESSED = "separator.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;
    /// Separator: size.
    SEPARATOR_SIZE_DEFAULT = "separator.size.default": Dimension = px(1.0), range(1.0, 10.0);
    /// Separator: text border width.
    SEPARATOR_TEXT_BORDER_WIDTH_DEFAULT = "separator.text.border.width.default": Dimension =
        px(3.0), range(0.0, 10.0);

    /// Scrollbar: background color.
    SCROLLBAR_BACKGROUND_COLOR_DEFAULT = "scrollbar.background.color.default": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Scrollbar: grab color.
    SCROLLBAR_GRAB_COLOR_DEFAULT = "scrollbar.grab.color.default": Color ->
        semantic::BORDER_COLOR_SUBTLE_DEFAULT;
    /// Scrollbar: grab color, hovered.
    SCROLLBAR_GRAB_COLOR_HOVER = "scrollbar.grab.color.hover": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Scrollbar: grab color, pressed.
    SCROLLBAR_GRAB_COLOR_PRESSED = "scrollbar.grab.color.pressed": Color ->
        semantic::TEXT_COLOR_TERTIARY_DEFAULT;
    /// Scrollbar: size.
    SCROLLBAR_SIZE_DEFAULT = "scrollbar.size.default": Dimension = px(14.0), range(1.0, 32.0);
    /// Scrollbar: corner radius.
    SCROLLBAR_CORNER_RADIUS_DEFAULT = "scrollbar.corner-radius.default": Dimension ->
        semantic::RADIUS_M;
    /// Scrollbar: padding.
    SCROLLBAR_PADDING_DEFAULT = "scrollbar.padding.default": Dimension = px(2.0), range(0.0, 10.0);
    /// Scrollbar: overlay margin.
    SCROLLBAR_OVERLAY_MARGIN_DEFAULT = "scrollbar.overlay.margin.default": Dimension =
        px(7.0), range(0.0, 20.0);
    /// Scrollbar: overlay width, at rest.
    SCROLLBAR_OVERLAY_WIDTH_REST = "scrollbar.overlay.width.rest": Dimension =
        px(3.0), range(0.0, 20.0);
    /// Scrollbar: overlay width, hovered.
    SCROLLBAR_OVERLAY_WIDTH_HOVER = "scrollbar.overlay.width.hover": Dimension =
        px(5.0), range(0.0, 20.0);
    /// Scrollbar: overlay proximity.
    SCROLLBAR_OVERLAY_PROXIMITY_DEFAULT = "scrollbar.overlay.proximity.default": Dimension =
        px(48.0), range(0.0, 200.0);
    /// Scrollbar: overlay padding.
    SCROLLBAR_OVERLAY_PADDING_DEFAULT = "scrollbar.overlay.padding.default": Dimension =
        px(4.0), range(0.0, 20.0);

    /// Tab: background color.
    TAB_BACKGROUND_COLOR_DEFAULT = "tab.background.color.default": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Tab: background color, hovered.
    TAB_BACKGROUND_COLOR_HOVER = "tab.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Tab: background color, selected.
    TAB_BACKGROUND_COLOR_SELECTED = "tab.background.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Tab: background color, in an unfocused window.
    TAB_BACKGROUND_COLOR_DIMMED = "tab.background.color.dimmed": Color ->
        semantic::BACKGROUND_BASE_DEFAULT;
    /// Tab: background color, selected, in an unfocused window.
    TAB_BACKGROUND_COLOR_DIMMED_SELECTED = "tab.background.color.dimmed-selected": Color ->
        semantic::BACKGROUND_SUNKEN_DEFAULT;
    /// Tab: overline color, selected.
    TAB_OVERLINE_COLOR_SELECTED = "tab.overline.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Tab: overline color, in an unfocused window.
    TAB_OVERLINE_COLOR_DIMMED = "tab.overline.color.dimmed": Color = rgba_f32(0.5, 0.5, 0.5, 0.0);
    /// Tab: corner radius.
    TAB_CORNER_RADIUS_DEFAULT = "tab.corner-radius.default": Dimension -> semantic::RADIUS_S;
    /// Tab: border width.
    TAB_BORDER_WIDTH_DEFAULT = "tab.border.width.default": Dimension -> semantic::BORDER_WIDTH_NONE;
    /// Tab: bar border width.
    TAB_BAR_BORDER_WIDTH_DEFAULT = "tab.bar.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Tab: bar overline width.
    TAB_BAR_OVERLINE_WIDTH_DEFAULT = "tab.bar.overline.width.default": Dimension =
        px(1.0), range(0.0, 8.0);
    /// Tab: unsaved marker color.
    TAB_UNSAVED_MARKER_COLOR_DEFAULT = "tab.unsaved-marker.color.default": Color ->
        semantic::TEXT_COLOR_PRIMARY_DEFAULT;

    /// Header: background color.
    HEADER_BACKGROUND_COLOR_DEFAULT = "header.background.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Header: background color, hovered.
    HEADER_BACKGROUND_COLOR_HOVER = "header.background.color.hover": Color ->
        semantic::BACKGROUND_ACCENT_HOVER;
    /// Header: background color, pressed.
    HEADER_BACKGROUND_COLOR_PRESSED = "header.background.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;

    /// Resize grip: color.
    RESIZE_GRIP_COLOR_DEFAULT = "resize-grip.color.default": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Resize grip: color, hovered.
    RESIZE_GRIP_COLOR_HOVER = "resize-grip.color.hover": Color -> semantic::BACKGROUND_ACCENT_HOVER;
    /// Resize grip: color, pressed.
    RESIZE_GRIP_COLOR_PRESSED = "resize-grip.color.pressed": Color ->
        semantic::BACKGROUND_ACCENT_PRESSED;

    /// Color swatch: corner radius.
    COLOR_SWATCH_CORNER_RADIUS_DEFAULT = "color-swatch.corner-radius.default": Dimension ->
        semantic::RADIUS_XS;
    /// Color swatch: border color.
    COLOR_SWATCH_BORDER_COLOR_DEFAULT = "color-swatch.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Color swatch: border color, hovered.
    COLOR_SWATCH_BORDER_COLOR_HOVER = "color-swatch.border.color.hover": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Color swatch: border color, selected.
    COLOR_SWATCH_BORDER_COLOR_SELECTED = "color-swatch.border.color.selected": Color ->
        semantic::BORDER_COLOR_FOCUS_DEFAULT;
    /// Color swatch: border width.
    COLOR_SWATCH_BORDER_WIDTH_DEFAULT = "color-swatch.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Color swatch: border width, selected.
    COLOR_SWATCH_BORDER_WIDTH_SELECTED = "color-swatch.border.width.selected": Dimension ->
        semantic::BORDER_WIDTH_MEDIUM;
    /// Color swatch: light squares of the checkerboard behind translucent colors.
    COLOR_SWATCH_CHECKER_COLOR_LIGHT = "color-swatch.checker.color.light": Color ->
        semantic::NEUTRAL_COLOR_500;
    /// Color swatch: dark squares of the checkerboard behind translucent colors.
    COLOR_SWATCH_CHECKER_COLOR_DARK = "color-swatch.checker.color.dark": Color ->
        semantic::NEUTRAL_COLOR_700;
    /// Color swatch: size of the checkerboard squares.
    COLOR_SWATCH_CHECKER_SIZE_DEFAULT = "color-swatch.checker.size.default": Dimension =
        px(6.0), range(2.0, 32.0);

    /// Theme card: corner radius.
    THEME_CARD_CORNER_RADIUS_DEFAULT = "theme-card.corner-radius.default": Dimension ->
        semantic::RADIUS_M;
    /// Theme card: border color.
    THEME_CARD_BORDER_COLOR_DEFAULT = "theme-card.border.color.default": Color ->
        semantic::BORDER_COLOR_DEFAULT;
    /// Theme card: border color, hovered.
    THEME_CARD_BORDER_COLOR_HOVER = "theme-card.border.color.hover": Color ->
        semantic::BORDER_COLOR_STRONG_DEFAULT;
    /// Theme card: border color of the active theme's card.
    THEME_CARD_BORDER_COLOR_SELECTED = "theme-card.border.color.selected": Color ->
        semantic::BACKGROUND_ACCENT_DEFAULT;
    /// Theme card: border width.
    THEME_CARD_BORDER_WIDTH_DEFAULT = "theme-card.border.width.default": Dimension ->
        semantic::BORDER_WIDTH_THIN;
    /// Theme card: border width of the active theme's card.
    THEME_CARD_BORDER_WIDTH_SELECTED = "theme-card.border.width.selected": Dimension ->
        semantic::BORDER_WIDTH_MEDIUM;
    /// Theme card: space between the border and the preview.
    THEME_CARD_PADDING_DEFAULT = "theme-card.padding.default": Dimension =
        px(4.0), range(0.0, 16.0);
}

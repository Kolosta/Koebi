//! Koebi's token set: validity, ported values, themes and contrast.

use core::time::Duration;

use koebi_core::Color;
use koebi_design::value::FontWeight;
use koebi_design::{DesignSystem, Layers, Origin, ThemeId};
use koebi_tokens::{KOEBI, component, primitive, semantic, themes};

fn design() -> DesignSystem {
    DesignSystem::new(&KOEBI).expect("Koebi's schema is valid")
}

fn theme(design: &DesignSystem, key: &str) -> ThemeId {
    design.find_theme(key).expect("built-in theme")
}

fn hex(rgb: u32) -> Color {
    Color::from_hex(rgb)
}

#[test]
fn schema_passes_the_run_time_validation() {
    assert_eq!(KOEBI.validate(), Ok(()));
    assert_eq!(KOEBI.len(), 1547);
    assert_eq!(KOEBI.composites().count(), 21);
    let keys: Vec<_> = design().themes().map(|(_, t)| t.key().to_owned()).collect();
    assert_eq!(keys, ["dark", "light", "muted-green", "high-contrast"]);
}

#[test]
fn every_token_resolves_in_every_theme() {
    let mut design = design();
    let ids: Vec<_> = design.themes().map(|(id, _)| id).collect();
    for id in ids {
        design.set_active_theme(id).unwrap();
        for (token, info) in KOEBI.tokens() {
            let value = design.value(token).unwrap();
            assert_eq!(value.kind(), info.kind(), "{}", info.qualified());
        }
    }
}

#[test]
fn primitive_values_are_ported() {
    let design = design();
    assert_eq!(design.get(primitive::COLOR_BLUE_700), hex(0x5D_89_FF));
    assert_eq!(design.get(primitive::COLOR_GRAY_875), hex(0x23_23_23));
    assert_eq!(
        design.get(primitive::COLOR_TRANSPARENT_BLACK_500),
        Color::from_hex_rgba(0x00_00_00_70)
    );
    assert_eq!(design.get(primitive::SPACING_300), 8.0);
    assert_eq!(design.get(primitive::UI_UNIT), 20.0);
    assert_eq!(design.get(primitive::RADIUS_FULL), 9999.0);
    assert_eq!(
        design.get(primitive::DURATION_NORMAL),
        Duration::from_millis(200)
    );
    assert_eq!(
        design.get(primitive::FONT_WEIGHT_SEMI_BOLD),
        FontWeight::new(600).unwrap()
    );
    assert_eq!(design.get(primitive::LAYER_ABSOLUTE), 9999);
    assert_eq!(&*design.get(primitive::FONT_FAMILY_SANS), "NotoSans");
}

#[test]
fn renamed_tokens_keep_their_values() {
    let design = design();
    // `font-size` joined the `font` family.
    assert_eq!(design.get(primitive::FONT_SIZE_60), 15.0);
    assert!(design.find("primitive.font.size.60").is_some());
    assert!(design.find("primitive.font-size.60").is_none());
    // `border-width` joined the `border` family.
    assert_eq!(design.get(semantic::BORDER_WIDTH_THIN), 1.0);
    // `down` became `pressed`.
    assert_eq!(
        design.get(component::BUTTON_BACKGROUND_COLOR_PRESSED),
        design.get(semantic::BACKGROUND_ACCENT_PRESSED)
    );
    // `accent.color.visual` moved beside the `accent.color.visual.*` group.
    assert_eq!(
        design.get(semantic::ACCENT_COLOR_VISUAL_BASE),
        design.get(semantic::ACCENT_COLOR_500)
    );
}

#[test]
fn semantic_aliases_follow_the_source() {
    let design = design();
    assert_eq!(
        design.get(semantic::BACKGROUND_ACCENT_DEFAULT),
        hex(0x5D_89_FF)
    );
    assert_eq!(
        design.get(semantic::BACKGROUND_BASE_DEFAULT),
        hex(0x13_13_13)
    );
    assert_eq!(
        design.get(semantic::TEXT_COLOR_PRIMARY_DEFAULT),
        hex(0xE9_E9_E9)
    );
    // The row highlight family is ported as is.
    assert_eq!(
        design.get(semantic::ACCENT_COLOR_HOVER_POSITIVE),
        design.get(semantic::POSITIVE_COLOR_700)
    );
    let body = design.typography(semantic::TEXT_BODY_M);
    assert_eq!(
        (&*body.font_family, body.font_size, body.font_weight.get()),
        ("NotoSans", 15.0, 400)
    );
    assert_eq!((body.line_height, body.letter_spacing), (1.5, 0.0));
}

#[test]
fn component_tokens_alias_or_hold_their_values() {
    let design = design();
    assert_eq!(
        design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT),
        design.get(semantic::BACKGROUND_ACCENT_DEFAULT)
    );
    assert_eq!(design.get(component::SCROLLBAR_SIZE_DEFAULT), 14.0);
    assert_eq!(design.get(component::CHECKBOX_BOX_SIZE_DEFAULT), 16.0);
    // Two-dimensional paddings became one token per axis.
    assert_eq!(
        design.get(component::WINDOW_PADDING_HORIZONTAL_DEFAULT),
        8.0
    );
    assert_eq!(design.get(component::WINDOW_PADDING_VERTICAL_DEFAULT), 4.0);
    assert_eq!(design.get(component::MENU_PADDING_VERTICAL_DEFAULT), 6.0);
    // Aliases to primitives became the primitive's value.
    assert_eq!(
        design.get(component::VIEWPORT_CURSOR_AXIS_X),
        design.get(primitive::COLOR_STATIC_RED_800)
    );
    assert_eq!(
        design.get(component::TAB_OVERLINE_COLOR_DIMMED),
        Color::new(0.5, 0.5, 0.5, 0.0)
    );
    assert_eq!(
        design.get(component::ZONE_TAB_DROP_PREVIEW_ANIM_DURATION),
        Duration::from_millis(60)
    );
}

#[test]
fn imgui_residues_are_not_ported() {
    let design = design();
    for path in [
        "primitive.config.item-spacing",
        "semantic.config.item-spacing",
        "component.window.size.min",
        "component.tab.close-button.min-width-selected",
        "component.docking.separator.size.default",
    ] {
        assert!(design.find(path).is_none(), "{path}");
    }
}

#[test]
fn themes_redefine_what_differs_from_the_base() {
    let mut design = design();
    let light = theme(&design, "light");
    design.set_active_theme(light).unwrap();
    assert_eq!(
        design.get(semantic::BACKGROUND_BASE_DEFAULT),
        hex(0xF8_F8_F8)
    );
    assert_eq!(
        design.get(component::BUTTON_LABEL_COLOR_DEFAULT),
        hex(0x13_13_13)
    );

    let muted = theme(&design, "muted-green");
    design.set_active_theme(muted).unwrap();
    assert_eq!(
        design.get(component::BUTTON_BACKGROUND_COLOR_DEFAULT),
        design.get(primitive::COLOR_GREEN_700)
    );
    assert_eq!(
        design.get(semantic::BACKGROUND_BASE_DEFAULT),
        hex(0x13_13_13)
    );

    let high = theme(&design, "high-contrast");
    design.set_active_theme(high).unwrap();
    assert_eq!(
        design.get(semantic::BACKGROUND_BASE_DEFAULT),
        hex(0x00_00_00)
    );
    assert_eq!(design.get(semantic::BORDER_WIDTH_THIN), 2.0);
    assert_eq!(design.get(component::CHECKBOX_BORDER_WIDTH_DEFAULT), 1.0);

    for (info, id) in [
        (themes::LIGHT, light),
        (themes::MUTED_GREEN, muted),
        (themes::HIGH_CONTRAST, high),
    ] {
        let view = design.view(id, Layers::THEME).unwrap();
        for entry in info.entries() {
            assert_eq!(view.origin(entry.token()), Origin::Theme);
        }
    }
    assert_eq!(
        (
            themes::LIGHT.entries().len(),
            themes::MUTED_GREEN.entries().len()
        ),
        (20, 16)
    );
    assert_eq!(themes::HIGH_CONTRAST.entries().len(), 24);
}

#[test]
fn text_meets_wcag_contrast_on_every_surface_in_every_theme() {
    let mut design = design();
    let surfaces = [
        semantic::BACKGROUND_BASE_DEFAULT,
        semantic::BACKGROUND_RAISED_DEFAULT,
        semantic::BACKGROUND_SUNKEN_DEFAULT,
        semantic::BACKGROUND_FLOATING_DEFAULT,
        semantic::BACKGROUND_APP_CONTROL,
        semantic::BACKGROUND_APP_FRAME,
        semantic::BACKGROUND_SURFACE_CANVAS,
    ];
    let ids: Vec<_> = design.themes().map(|(id, _)| id).collect();
    for id in ids {
        design.set_active_theme(id).unwrap();
        for surface in surfaces {
            let background = design.get(surface);
            // Body text needs 4.5:1 (AA).
            for text in [
                semantic::TEXT_COLOR_PRIMARY_DEFAULT,
                semantic::TEXT_COLOR_SECONDARY_DEFAULT,
                semantic::ICON_COLOR_PRIMARY_DEFAULT,
            ] {
                let ratio = design.get(text).contrast_ratio(background);
                assert!(ratio >= 4.5, "{id}: {text:?} on {surface:?}: {ratio:.2}");
            }
            // Hints may be large-text only: 3:1.
            for hint in [
                semantic::TEXT_COLOR_TERTIARY_DEFAULT,
                semantic::ICON_COLOR_SECONDARY_DEFAULT,
            ] {
                let ratio = design.get(hint).contrast_ratio(background);
                assert!(ratio >= 3.0, "{id}: {hint:?} on {surface:?}: {ratio:.2}");
            }
        }
    }
}

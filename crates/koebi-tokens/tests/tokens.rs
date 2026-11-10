//! Koebi's token set: validity, ported values, themes and contrast.

use core::time::Duration;

use koebi_core::Color;
use koebi_design::value::{Definition, FontWeight, Value};
use koebi_design::{ChainRole, DesignSystem, Layers, Origin, Scope, ThemeId};
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
    assert_eq!(KOEBI.len(), 1940);
    assert_eq!(KOEBI.composites().count(), 21);
    let keys: Vec<_> = design().themes().map(|(_, t)| t.key().to_owned()).collect();
    assert_eq!(
        keys,
        [
            "dark",
            "dark-pink",
            "muted-green",
            "snow",
            "arctic",
            "high-contrast"
        ]
    );
}

#[test]
fn sizes_that_vanish_at_zero_refuse_it() {
    let mut design = design();
    // At zero, each of these draws nothing or leaves controls without room.
    let sizes = [
        primitive::SIZE_75.id(),
        primitive::SIZE_800.id(),
        semantic::SIZE_M.id(),
        semantic::SIZE_CONTROL_HEIGHT.id(),
        component::CHECKBOX_BOX_SIZE_DEFAULT.id(),
        component::DROPDOWN_CHEVRON_SIZE.id(),
        component::DROPDOWN_ICON_SIZE.id(),
        component::NUMBER_FIELD_STEP_BUTTON_WIDTH.id(),
        component::NUMBER_FIELD_STEP_BUTTON_ICON_SIZE.id(),
        component::TEXT_FIELD_ICON_SIZE.id(),
        component::TEXT_FIELD_CARET_WIDTH.id(),
        component::SCROLLBAR_OVERLAY_WIDTH_HOVER.id(),
        component::PANEL_CHEVRON_SIZE.id(),
        component::NODE_PORT_SIZE_DEFAULT.id(),
        component::NODE_CABLE_WIDTH_DEFAULT.id(),
        component::CURVE_EDITOR_GRAPH_HEIGHT_DEFAULT.id(),
        component::COLOR_PICKER_AREA_SIZE_DEFAULT.id(),
    ];
    for token in sizes {
        let path = design
            .info(token)
            .expect("a token of the schema")
            .qualified();
        let range = design.range(token).expect("numeric tokens have bounds");
        assert!(range.min() > 0.0, "{path} accepts {}", range.min());
        // Refused with an error, the value kept.
        let before = design.value(token).cloned();
        let zero = Definition::Value(Value::Dimension(0.0));
        assert!(
            design.set_override(Scope::Global, token, zero).is_err(),
            "{path}"
        );
        assert_eq!(design.value(token).cloned(), before, "{path}");
    }
}

#[test]
fn features_hold_every_group_once_in_schema_order() {
    // Groups by where they start: names repeat across tiers (`SCALE`, `MOTION`).
    let features: Vec<u16> = koebi_tokens::FEATURES
        .iter()
        .flat_map(|feature| {
            feature
                .groups
                .iter()
                .map(koebi_design::schema::Group::start)
        })
        .collect();
    // Components first, then intents, then primitives, each tier in schema order.
    let mut schema: Vec<_> = KOEBI.groups().iter().collect();
    schema.sort_by_key(|group| {
        core::cmp::Reverse(
            group
                .tokens()
                .first()
                .map(koebi_design::schema::TokenInfo::tier),
        )
    });
    let schema: Vec<u16> = schema.iter().map(|group| group.start()).collect();
    assert_eq!(features, schema);
}

#[test]
fn every_numeric_token_has_bounds() {
    let design = design();
    let unbounded: Vec<String> = KOEBI
        .tokens()
        .filter(|(token, info)| info.kind().is_numeric() && design.range(*token).is_none())
        .map(|(_, info)| info.qualified().to_string())
        .collect();
    assert_eq!(unbounded, Vec::<String>::new());
}

#[test]
fn built_in_themes_stay_inside_every_range() {
    // Resolved values are kept inside each token's range; the built-in themes never rely on
    // it: a token's own range accepts what it aliases.
    let mut design = design();
    let ids: Vec<_> = design.themes().map(|(id, _)| id).collect();
    let mut outside = Vec::new();
    for id in ids {
        design.set_active_theme(id).unwrap();
        let view = design.view(id, Layers::ALL).unwrap();
        for (token, info) in KOEBI.tokens() {
            let Some(range) = design.range(token) else {
                continue;
            };
            // The value before limiting: the one at the end of the alias chain.
            let mut current = token;
            let raw = loop {
                match view.definition(current) {
                    Some(Definition::Alias(target)) => current = *target,
                    Some(Definition::Value(value)) => break value.clone(),
                    None => unreachable!("every token has a definition"),
                }
            };
            let number = match &raw {
                Value::Color(color) => Some(f64::from(color.a)),
                other => other.as_number(),
            };
            if let Some(number) = number
                && !range.contains(number)
            {
                outside.push(format!("{} = {number} ({range})", info.qualified()));
            }
        }
    }
    assert_eq!(outside, Vec::<String>::new());
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
    // The grays of the dark theme keep their values on the contrast-ordered steps.
    for (token, value) in [
        (primitive::COLOR_GRAY_50, 0x13_13_13),
        (primitive::COLOR_GRAY_100, 0x23_23_23),
        (primitive::COLOR_GRAY_150, 0x29_29_29),
        (primitive::COLOR_GRAY_300, 0x48_48_48),
        (primitive::COLOR_GRAY_900, 0xE9_E9_E9),
    ] {
        assert_eq!(design.get(token), hex(value), "{token:?}");
    }
    // Spectrum's palettes are the keys of the hues' scales.
    let blue = koebi_tokens::palettes::HUES
        .iter()
        .find(|hue| hue.name == "blue")
        .unwrap();
    assert_eq!(blue.keys[6], 0x5D_89_FF);
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
    // Controls and surfaces have no border in the base theme; floating surfaces keep it.
    assert_eq!(design.get(semantic::BORDER_WIDTH_CONTROL), 0.0);
    assert_eq!(design.get(semantic::BORDER_WIDTH_SURFACE), 0.0);
    assert_eq!(design.get(semantic::BORDER_WIDTH_FLOATING), 1.0);
    // `down` became `pressed` (the button's own value changed later: its default variant is a
    // neutral control, see the porting report).
    assert_eq!(
        design.get(component::CHECKBOX_BACKGROUND_COLOR_PRESSED),
        design.get(semantic::BACKGROUND_APP_CHILD)
    );
    assert!(
        design
            .find("component.button.background.color.pressed")
            .is_some()
    );
    // `accent.color.visual` moved beside the `accent.color.visual.*` group.
    assert_eq!(
        design.get(semantic::ACCENT_COLOR_VISUAL_BASE),
        design.get(semantic::ACCENT_COLOR_1300)
    );
}

#[test]
fn semantic_aliases_follow_the_source() {
    let design = design();
    // Accent fills hold white text (#60): step 900.
    assert_eq!(
        design.get(semantic::BACKGROUND_ACCENT_DEFAULT),
        design.get(primitive::COLOR_BLUE_900)
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
        design.get(semantic::POSITIVE_COLOR_400)
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
        design.get(component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT),
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
    let snow = theme(&design, "snow");
    design.set_active_theme(snow).unwrap();
    assert_eq!(
        design.get(semantic::BACKGROUND_BASE_DEFAULT),
        hex(0xF8_F5_F3)
    );
    // The same step, the theme's own color: dark text on the light background.
    assert!(
        design
            .get(semantic::TEXT_COLOR_PRIMARY_DEFAULT)
            .relative_luminance()
            < 0.05
    );
    assert_eq!(design.get(semantic::BORDER_WIDTH_CONTROL), 1.0);
    assert_eq!(
        design.get(semantic::BACKGROUND_ACCENT_DEFAULT),
        design.get(primitive::COLOR_CINNAMON_900)
    );

    let muted = theme(&design, "muted-green");
    design.set_active_theme(muted).unwrap();
    assert_eq!(
        design.get(component::BUTTON_ACCENT_BACKGROUND_COLOR_DEFAULT),
        design.get(primitive::COLOR_CELERY_900)
    );
    assert_eq!(
        design.get(semantic::BACKGROUND_BASE_DEFAULT),
        hex(0x16_1A_14)
    );
    // Outlined controls, as before the base theme dropped them.
    assert_eq!(design.get(semantic::BORDER_WIDTH_CONTROL), 1.0);

    let pink = theme(&design, "dark-pink");
    design.set_active_theme(pink).unwrap();
    assert_eq!(
        design.get(semantic::BACKGROUND_ACCENT_DEFAULT),
        design.get(primitive::COLOR_PINK_900)
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
    assert_eq!(design.get(semantic::BORDER_WIDTH_CONTROL), 2.0);
    assert_eq!(design.get(component::CHECKBOX_BORDER_WIDTH_DEFAULT), 1.0);

    let arctic = theme(&design, "arctic");
    for (info, id) in [
        (themes::SNOW, snow),
        (themes::ARCTIC, arctic),
        (themes::MUTED_GREEN, muted),
        (themes::DARK_PINK, pink),
        (themes::HIGH_CONTRAST, high),
    ] {
        let view = design.view(id, Layers::THEME).unwrap();
        for entry in info.entries() {
            assert_eq!(view.origin(entry.token()), Origin::Theme);
        }
    }
    // Dark pink changes the accent only; the others their palettes too.
    assert_eq!(themes::DARK_PINK.entries().len(), 16);
    assert!(themes::SNOW.entries().len() > 16 + 18 * 16);
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

#[test]
fn every_theme_round_trips_through_the_design_tokens_format() {
    use koebi_design::dtcg::{export_schema, export_theme, import_theme};

    let mut design = design();
    let values = |design: &DesignSystem, theme: ThemeId, layers: Layers| {
        let view = design.view(theme, layers).unwrap();
        KOEBI
            .tokens()
            .map(|(id, _)| view.value(id).unwrap())
            .collect::<Vec<_>>()
    };
    let builtin: Vec<_> = design
        .themes()
        .map(|(id, t)| (id, t.key().to_owned()))
        .collect();
    for (id, key) in builtin {
        let text = export_theme(&design, id).unwrap();
        let copy = text.replace(
            &format!("\"key\": \"{key}\""),
            &format!("\"key\": \"{key}-copy\""),
        );
        let imported = import_theme(&mut design, &copy).unwrap();
        assert_eq!(
            values(&design, imported, Layers::THEME),
            values(&design, id, Layers::THEME),
            "{key}"
        );
    }

    // The full schema export is a valid theme too: importing it reproduces the base theme.
    let schema = export_schema(&design).replace(
        "\"base\": \"dark\"",
        "\"base\": \"dark\",\n      \"theme\": { \"key\": \"schema\", \"name\": \"Schema\" }",
    );
    let imported = import_theme(&mut design, &schema).unwrap();
    assert_eq!(
        values(&design, imported, Layers::THEME),
        values(&design, ThemeId::BASE, Layers::NONE)
    );
}

#[test]
fn a_chain_goes_down_to_the_token_holding_the_value() {
    let design = design();
    let view = design.view(design.active_theme(), Layers::ALL).unwrap();
    let mut steps = Vec::new();
    view.chain(component::NUMBER_FIELD_FILL_COLOR_DEFAULT, &mut steps);
    assert!(steps.len() >= 3, "{steps:?}");
    assert_eq!(
        steps[0].token,
        component::NUMBER_FIELD_FILL_COLOR_DEFAULT.id()
    );
    let (source, passing) = steps.split_last().unwrap();
    assert_eq!(source.role, ChainRole::Source);
    assert!(passing.iter().all(|step| step.role == ChainRole::Passes));
    // Each step shows the value it gives.
    assert!(steps.iter().all(|step| step.value == steps[0].value));
}

#[test]
fn an_override_gives_the_value_and_what_it_replaced_stays_in_the_chain() {
    let mut design = design();
    let token = semantic::INPUT_DRAG_THRESHOLD;
    design.set_value(Scope::Global, token, 9.0).unwrap();
    let view = design.view(design.active_theme(), Layers::ALL).unwrap();
    let mut steps = Vec::new();
    view.chain(token, &mut steps);
    assert_eq!(steps[0].role, ChainRole::Source);
    assert_eq!(steps[0].origin, Origin::GlobalOverride);
    assert_eq!(steps[0].value, Some(Value::Dimension(9.0)));
    // The token it refers to by default has no effect, and keeps its own value.
    assert_eq!(steps[1].role, ChainRole::Unused);
    assert_eq!(steps[1].origin, Origin::Base);
    assert_ne!(steps[1].value, steps[0].value);
    assert_eq!(steps.len(), 2);
}

#[test]
fn an_override_along_the_way_stops_the_values_there() {
    let mut design = design();
    let component = component::NUMBER_FIELD_DRAG_STEP_DISTANCE;
    let semantic_token = semantic::INPUT_TOOLTIP_DELAY;
    // A token holding its own value: nothing past it.
    let view = design.view(design.active_theme(), Layers::ALL).unwrap();
    let mut steps = Vec::new();
    view.chain(component, &mut steps);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].role, ChainRole::Source);
    // An alias overridden in the theme: the theme's override is the source.
    design
        .set_value(
            Scope::ActiveTheme,
            semantic_token,
            Duration::from_millis(50),
        )
        .unwrap();
    let view = design.view(design.active_theme(), Layers::ALL).unwrap();
    view.chain(semantic_token, &mut steps);
    assert_eq!(steps[0].origin, Origin::ThemeOverride);
    assert_eq!(steps[0].role, ChainRole::Source);
    assert_eq!(steps[1].role, ChainRole::Unused);
}

#[test]
fn toggles_take_the_border_width_of_the_button_groups_beside_them() {
    // Regression: the shortcut editor's Any, a toggle, kept a 1 px border of its own where the
    // button groups beside it follow `semantic.border.width.thin`, which a theme may set to 0.
    let mut design = design();
    let ids: Vec<_> = design.themes().map(|(id, _)| id).collect();
    for id in ids {
        design.set_active_theme(id).unwrap();
        for thin in [None, Some(0.0), Some(2.0)] {
            if let Some(width) = thin {
                design
                    .set_value(Scope::ActiveTheme, semantic::BORDER_WIDTH_THIN, width)
                    .unwrap();
            }
            assert_eq!(
                design.get(component::TOGGLE_BORDER_WIDTH_DEFAULT),
                design.get(component::BUTTON_GROUP_BORDER_WIDTH_DEFAULT),
                "{id}, thin border {thin:?}"
            );
        }
        design
            .reset(Scope::ActiveTheme, semantic::BORDER_WIDTH_THIN)
            .unwrap();
    }
}

#[test]
fn tooltips_stand_out_in_the_colors_of_their_theme() {
    // Tooltips were inverted (white in the dark themes); they keep the theme's colors and
    // stand out by being deeper than what they cover, with a border on the window background.
    let mut design = design();
    let ids: Vec<_> = design.themes().map(|(id, _)| id).collect();
    for id in ids {
        design.set_active_theme(id).unwrap();
        let background = design.get(component::TOOLTIP_BACKGROUND_COLOR_DEFAULT);
        let label = design.get(component::TOOLTIP_LABEL_COLOR_DEFAULT);
        let ratio = label.contrast_ratio(background);
        assert!(ratio >= 4.5, "{id}: tooltip text {ratio:.2}");
        // Dark themes keep a dark tooltip.
        let base = design.get(semantic::BACKGROUND_BASE_DEFAULT);
        assert_eq!(
            background.relative_luminance() < 0.2,
            base.relative_luminance() < 0.2,
            "{id}: the tooltip follows the theme's lightness"
        );
        for covered in [
            semantic::BACKGROUND_RAISED_DEFAULT,
            semantic::BACKGROUND_FLOATING_DEFAULT,
            semantic::BACKGROUND_APP_FRAME,
        ] {
            assert_ne!(design.get(covered), background, "{id}: {covered:?}");
        }
        assert_ne!(
            design.get(component::TOOLTIP_BORDER_COLOR_DEFAULT),
            base,
            "{id}: the border shows on the window background"
        );
    }
}

//! Icons compiled and rasterized: the forms of the previous application's icons (Material's view
//! box, a root fill, classes), the space an icon keeps on one side, strokes, gradients, the
//! even-odd rule, zones and their roles, what the subset refuses, and a coverage without seams
//! where shapes overlap that keeps the weight of features thinner than a pixel.

use std::fmt::Write as _;

use koebi_svg::raster::{Colors, render};
use koebi_svg::{FillRule, Paint, Rgba, Role, compile};

/// The alpha of the pixel at `(x, y)` of a `size` square render.
fn alpha(pixels: &[u8], size: u32, x: u32, y: u32) -> u8 {
    pixels[((y * size + x) * 4 + 3) as usize]
}

fn rgb(pixels: &[u8], size: u32, x: u32, y: u32) -> [u8; 3] {
    let at = ((y * size + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

#[test]
fn material_icons_compile_with_their_root_fill_and_class() {
    // `close.svg` as the previous application had it.
    let icon = compile(
        r##"<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="#e3e3e3"><path class="ds-primary" d="m256-200-56-56 224-224-224-224 56-56 224 224 224-224 56 56-224 224 224 224-56 56-224-224-224 224Z"/></svg>"##,
    )
    .unwrap();
    assert_eq!(icon.layers.len(), 1);
    assert_eq!(icon.zones.len(), 1);
    assert_eq!(icon.zones[0].role, Role::Primary);
    assert_eq!(icon.zones[0].color.to_rgba8(), [227, 227, 227, 255]);
    let pixels = render(&icon, 48, Colors::Original);
    // The cross's center is painted, its corners' gaps are not.
    assert_eq!(alpha(&pixels, 48, 24, 24), 255);
    assert_eq!(alpha(&pixels, 48, 24, 10), 0);
    assert_eq!(rgb(&pixels, 48, 24, 24), [227, 227, 227]);
}

#[test]
fn an_icon_keeps_the_space_its_view_box_leaves() {
    // `align-justify-flex-end.svg`: its bars stand against the right, the left third empty.
    let icon = compile(
        r##"<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 -960 960 960" width="24px" fill="#e3e3e3"><path d="M800-80v-800h80v800h-80ZM560-280v-400h120v400H560Zm-240 0v-400h120v400H320Z"/></svg>"##,
    )
    .unwrap();
    let pixels = render(&icon, 24, Colors::Original);
    for x in 0..7 {
        for y in 0..24 {
            assert_eq!(alpha(&pixels, 24, x, y), 0, "({x}, {y})");
        }
    }
    // The tall bar at the right: x from 800 to 880 of 960.
    assert_eq!(alpha(&pixels, 24, 21, 12), 255);
}

#[test]
fn a_view_box_that_is_not_square_is_centered_in_the_square() {
    let icon = compile(r#"<svg viewBox="0 0 20 10"><rect width="20" height="10"/></svg>"#).unwrap();
    let pixels = render(&icon, 20, Colors::Original);
    // Ten pixels high, five above and below.
    assert_eq!(alpha(&pixels, 20, 10, 2), 0);
    assert_eq!(alpha(&pixels, 20, 10, 10), 255);
    assert_eq!(alpha(&pixels, 20, 10, 17), 0);
}

#[test]
fn strokes_become_fills_with_their_caps() {
    let icon = compile(
        r##"<svg viewBox="0 0 24 24"><path d="M4 12H20" fill="none" stroke="#fff" stroke-width="4" stroke-linecap="round"/></svg>"##,
    )
    .unwrap();
    assert_eq!(icon.layers.len(), 1);
    assert_eq!(icon.layers[0].rule, FillRule::NonZero);
    let pixels = render(&icon, 24, Colors::Original);
    assert_eq!(alpha(&pixels, 24, 12, 12), 255);
    // The round cap passes the end by half the width.
    assert!(alpha(&pixels, 24, 2, 12) > 200);
    assert_eq!(alpha(&pixels, 24, 12, 6), 0);
}

#[test]
fn gradients_go_from_stop_to_stop() {
    let icon = compile(
        r##"<svg viewBox="0 0 10 10">
          <defs><linearGradient id="shade"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs>
          <rect width="10" height="10" fill="url(#shade)"/>
        </svg>"##,
    )
    .unwrap();
    assert!(matches!(
        icon.layers[0].paint,
        Paint::Gradient { radial: false, .. }
    ));
    assert_eq!(icon.zones.len(), 2);
    let pixels = render(&icon, 10, Colors::Original);
    let [left_red, _, left_blue] = rgb(&pixels, 10, 0, 5);
    let [right_red, _, right_blue] = rgb(&pixels, 10, 9, 5);
    assert!(left_red > 200 && left_blue < 40, "{left_red} {left_blue}");
    assert!(
        right_blue > 200 && right_red < 40,
        "{right_red} {right_blue}"
    );
}

#[test]
fn even_odd_leaves_the_middle_of_a_ring_empty() {
    let icon = compile(
        r#"<svg viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 1a9 9 0 1 1 0 18a9 9 0 1 1 0-18ZM10 5a5 5 0 1 1 0 10a5 5 0 1 1 0-10Z"/></svg>"#,
    )
    .unwrap();
    let pixels = render(&icon, 20, Colors::Original);
    assert_eq!(alpha(&pixels, 20, 10, 10), 0);
    assert_eq!(alpha(&pixels, 20, 10, 3), 255);
}

#[test]
fn zones_take_their_role_from_classes_and_recolor() {
    let icon = compile(
        r##"<svg viewBox="0 0 20 10">
          <rect class="ds-secondary" width="10" height="10" fill="#ea4335"/>
          <rect class="ds-primary" x="10" width="10" height="10" fill="#1a73e8" fill-opacity="0.5"/>
        </svg>"##,
    )
    .unwrap();
    // Primary first, whatever the order of the file.
    assert_eq!(icon.zones[0].role, Role::Primary);
    assert_eq!(icon.zones[0].color.to_rgba8(), [26, 115, 232, 255]);
    assert_eq!(icon.zones[1].role, Role::Secondary);
    let white = Rgba([1.0, 1.0, 1.0, 1.0]);
    let black = Rgba([0.0, 0.0, 0.0, 1.0]);
    let pixels = render(&icon, 20, Colors::Zones(&[white, black]));
    // The primary zone white, keeping its opacity; the secondary black.
    assert_eq!(rgb(&pixels, 20, 15, 10), [255, 255, 255]);
    assert!((i32::from(alpha(&pixels, 20, 15, 10)) - 128).abs() <= 1);
    assert_eq!(rgb(&pixels, 20, 5, 10), [0, 0, 0]);
}

#[test]
fn what_the_subset_does_not_hold_is_refused_with_its_line() {
    let cases = [
        (
            "<svg viewBox=\"0 0 1 1\">\n<text>A</text>\n</svg>",
            2,
            "text",
        ),
        (
            "<svg viewBox=\"0 0 1 1\">\n\n<use href=\"#a\"/>\n</svg>",
            3,
            "use",
        ),
        (
            "<svg viewBox=\"0 0 1 1\">\n<path d=\"M0 0H1\" stroke=\"#000\" stroke-dasharray=\"1 1\"/>\n</svg>",
            2,
            "dash",
        ),
        (
            "<svg viewBox=\"0 0 1 1\"><path d=\"M0 0H1\" mask=\"url(#m)\"/></svg>",
            1,
            "mask",
        ),
        ("<svg width=\"0\"><path d=\"M0 0H1\"/></svg>", 1, "viewBox"),
        ("<svg viewBox=\"0 0 1 1\"></svg>", 0, "nothing"),
        (
            "<svg viewBox=\"0 0 1 1\"><path d=\"L1 1\"/></svg>",
            1,
            "move",
        ),
    ];
    for (text, line, word) in cases {
        let error = compile(text).unwrap_err();
        assert_eq!(error.line, line, "{error}");
        assert!(error.to_string().contains(word), "{error}");
    }
    // Nine colors: more zones than the renderer recolors.
    let mut many = String::from("<svg viewBox=\"0 0 9 1\">");
    for index in 0..9 {
        write!(
            many,
            "<rect x=\"{index}\" width=\"1\" height=\"1\" fill=\"#0{index}0000\"/>"
        )
        .unwrap();
    }
    many.push_str("</svg>");
    assert!(compile(&many).unwrap_err().message.contains("colors"));
}

#[test]
fn overlapping_shapes_show_no_seam() {
    // Two squares of one path overlapping: the edges of each inside the other are no edges of
    // the shape, and the pixels along them are fully covered.
    let icon = compile(r#"<svg viewBox="0 0 20 20"><path d="M2 2H12V12H2ZM8 8H18V18H8Z"/></svg>"#)
        .unwrap();
    let pixels = render(&icon, 20, Colors::Original);
    for (x, y) in [(7, 9), (8, 9), (9, 11), (9, 12), (11, 10), (12, 10)] {
        assert_eq!(alpha(&pixels, 20, x, y), 255, "({x}, {y})");
    }
}

#[test]
fn features_thinner_than_a_pixel_keep_their_weight() {
    // A bar a quarter of a pixel high, inside one row: a quarter covered, not half.
    let icon =
        compile(r#"<svg viewBox="0 0 8 8"><rect x="1" y="4.375" width="6" height="0.25"/></svg>"#)
            .unwrap();
    let pixels = render(&icon, 8, Colors::Original);
    let covered = i32::from(alpha(&pixels, 8, 4, 4));
    assert!((covered - 64).abs() <= 1, "{covered}");
    assert_eq!(alpha(&pixels, 8, 4, 3), 0);
    assert_eq!(alpha(&pixels, 8, 4, 5), 0);
}

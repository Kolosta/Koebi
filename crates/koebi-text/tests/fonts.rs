//! Integration tests of font reading, rasterization and layout on the embedded Noto Sans.
//!
//! Expected values were read from the font files with an independent parser.

use koebi_core::{Rect, Vec2};
use koebi_text::{
    Error, Font, Fonts, GlyphBitmap, GlyphId, Outline, PathCommand, Rasterizer, Tag, TextLine,
    TextStyle, embedded,
};

fn regular() -> Font {
    Font::from_static(embedded::NOTO_SANS_REGULAR).expect("the embedded font is valid")
}

fn medium() -> Font {
    Font::from_static(embedded::NOTO_SANS_MEDIUM).expect("the embedded font is valid")
}

fn glyph(font: &Font, c: char) -> GlyphId {
    font.glyph_index(c)
        .expect("Noto Sans covers the characters of these tests")
}

#[test]
fn embedded_fonts_read_their_metrics_and_weights() {
    let regular = regular();
    let metrics = regular.metrics();
    assert_eq!(
        (
            metrics.units_per_em,
            metrics.ascender,
            metrics.descender,
            metrics.line_gap
        ),
        (1000, 1069, -293, 0)
    );
    assert_eq!(regular.glyph_count(), 4503);
    assert_eq!(regular.weight(), 400);
    assert_eq!(medium().weight(), 500);
}

#[test]
fn characters_map_to_the_glyphs_of_the_font() {
    let font = regular();
    let expected = [
        ('A', 36),
        ('H', 43),
        (' ', 3),
        ('\u{a0}', 98),
        ('é', 171),
        ('€', 538),
        ('…', 526),
        ('Ж', 430),
    ];
    for (c, index) in expected {
        assert_eq!(font.glyph_index(c), Some(GlyphId(index)), "{c:?}");
    }
    // Arrows, emoji and CJK are not in Noto Sans.
    for missing in ['→', '😀', '中'] {
        assert_eq!(font.glyph_index(missing), None, "{missing:?}");
    }
}

#[test]
fn advances_follow_the_horizontal_metrics() {
    let font = regular();
    for (c, advance) in [('A', 639), ('V', 600), ('H', 741), (' ', 260), ('…', 791)] {
        assert_eq!(font.advance(glyph(&font, c)), advance, "{c:?}");
    }
    assert_eq!(font.advance(GlyphId::NOTDEF), 600);
    let medium = medium();
    assert_eq!(medium.advance(glyph(&medium, 'A')), 655);
    assert_eq!(medium.advance(glyph(&medium, '…')), 810);
}

fn contours(outline: &Outline) -> usize {
    outline
        .commands()
        .iter()
        .filter(|command| matches!(command, PathCommand::MoveTo(_)))
        .count()
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::from_min_max(Vec2::new(x0, y0), Vec2::new(x1, y1))
}

#[test]
fn outlines_follow_the_glyph_contours() {
    let font = regular();
    let mut outline = Outline::new();
    outline.load(&font, glyph(&font, 'H')).unwrap();
    assert_eq!(contours(&outline), 1);
    assert_eq!(outline.bounds(), Some(rect(97.0, 0.0, 643.0, 714.0)));
    // A straight-sided glyph: twelve points, twelve lines back to the start.
    assert!(
        outline.commands()[1..13]
            .iter()
            .all(|command| matches!(command, PathCommand::LineTo(_)))
    );

    outline.load(&font, glyph(&font, 'o')).unwrap();
    assert_eq!(contours(&outline), 2);
    assert_eq!(outline.bounds(), Some(rect(55.0, -10.0, 551.0, 546.0)));

    outline.load(&font, glyph(&font, ' ')).unwrap();
    assert!(outline.is_empty());
    assert_eq!(outline.bounds(), None);
}

#[test]
fn composite_glyphs_place_their_components() {
    let font = regular();
    let mut outline = Outline::new();
    // "é" is "e" (glyph 72) at the origin plus the acute accent (glyph 118) moved right by 192.
    outline.load(&font, glyph(&font, 'é')).unwrap();
    let composite = outline.commands().to_vec();
    outline.load(&font, GlyphId(72)).unwrap();
    let base = outline.commands().to_vec();
    outline.load(&font, GlyphId(118)).unwrap();
    let shift = Vec2::new(192.0, 0.0);
    let accent: Vec<_> = outline
        .commands()
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo(p) => PathCommand::MoveTo(p + shift),
            PathCommand::LineTo(p) => PathCommand::LineTo(p + shift),
            PathCommand::QuadTo(c, p) => PathCommand::QuadTo(c + shift, p + shift),
            PathCommand::Close => PathCommand::Close,
        })
        .collect();
    assert_eq!(composite, [base, accent].concat());
    outline.load(&font, glyph(&font, 'é')).unwrap();
    assert_eq!(outline.bounds(), Some(rect(55.0, -10.0, 513.0, 766.0)));
}

/// Area enclosed by a straight-sided outline (shoelace formula).
fn polygon_area(outline: &Outline) -> f32 {
    let mut area = 0.0;
    let (mut start, mut current) = (Vec2::default(), Vec2::default());
    for command in outline.commands() {
        match *command {
            PathCommand::MoveTo(p) => (start, current) = (p, p),
            // Straight-sided outlines have no curve; a chord would do for the others.
            PathCommand::LineTo(p) | PathCommand::QuadTo(_, p) => {
                area += current.x * p.y - p.x * current.y;
                current = p;
            }
            PathCommand::Close => {
                area += current.x * start.y - start.x * current.y;
                current = start;
            }
        }
    }
    (area * 0.5).abs()
}

fn total_coverage(bitmap: &GlyphBitmap) -> f32 {
    bitmap.coverage.iter().map(|c| f32::from(*c)).sum::<f32>() / 255.0
}

#[test]
fn rasterized_glyphs_cover_exactly_their_area() {
    let font = regular();
    let h = glyph(&font, 'H');
    let mut rasterizer = Rasterizer::new();
    let mut bitmap = GlyphBitmap::default();
    rasterizer
        .rasterize(&font, h, 100.0, 0.0, &mut bitmap)
        .unwrap();
    // Bounds 97..643 × 0..714 font units at 0.1 pixel per unit, y down from the baseline.
    assert_eq!(
        (bitmap.left, bitmap.top, bitmap.width, bitmap.height),
        (9, -72, 56, 72)
    );
    assert_eq!(bitmap.coverage.len(), 56 * 72);
    let mut outline = Outline::new();
    outline.load(&font, h).unwrap();
    let area = polygon_area(&outline) * 0.01;
    // Each pixel rounds to the nearest 1/255.
    let tolerance = (56 * 72) as f32 * 0.5 / 255.0;
    assert!(
        (total_coverage(&bitmap) - area).abs() <= tolerance,
        "{} vs {area}",
        total_coverage(&bitmap)
    );
    // The middle of the left stem is solid, the gap between the stems above the bar empty,
    // and the first column, from x = 9.7, three-tenths covered.
    assert_eq!(bitmap.coverage_at(4, 36), 255);
    assert_eq!(bitmap.coverage_at(28, 5), 0);
    assert!((bitmap.coverage_at(0, 36)).abs_diff(77) <= 1);

    // A quarter-pixel offset moves coverage right without changing its total.
    let mut shifted = GlyphBitmap::default();
    rasterizer
        .rasterize(&font, h, 100.0, 0.25, &mut shifted)
        .unwrap();
    assert_eq!(shifted.left, 9);
    assert!(shifted.coverage_at(0, 36) < bitmap.coverage_at(0, 36));
    assert!((total_coverage(&shifted) - area).abs() <= tolerance);
}

#[test]
fn spaces_and_invalid_requests_rasterize_to_nothing() {
    let font = regular();
    let mut rasterizer = Rasterizer::new();
    let mut bitmap = GlyphBitmap::default();
    rasterizer
        .rasterize(&font, glyph(&font, ' '), 14.0, 0.0, &mut bitmap)
        .unwrap();
    assert!(bitmap.is_empty());
    let h = glyph(&font, 'H');
    for (size, offset) in [(f32::NAN, 0.0), (-3.0, 0.0), (14.0, f32::INFINITY)] {
        let refused = rasterizer.rasterize(&font, h, size, offset, &mut bitmap);
        assert_eq!(refused, Err(Error::TooComplex));
        assert!(bitmap.is_empty());
    }
    // Bitmaps larger than the limit are refused rather than allocated.
    let huge = rasterizer.rasterize(&font, h, 5000.0, 0.0, &mut bitmap);
    assert_eq!(huge, Err(Error::TooComplex));
    assert_eq!(
        rasterizer.rasterize(&font, GlyphId(9999), 14.0, 0.0, &mut bitmap),
        Err(Error::UnknownGlyph(9999))
    );
}

fn style(fonts: &Fonts, weight: u16, size: f32) -> TextStyle {
    TextStyle {
        font: fonts
            .select(embedded::NOTO_SANS, weight)
            .expect("Noto Sans is embedded"),
        size,
        tracking: 0.0,
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn lines_place_glyphs_by_their_advances() {
    let fonts = Fonts::with_embedded();
    let mut line = TextLine::new();
    let regular = style(&fonts, 400, 10.0);
    line.layout(&fonts, "AV", &regular);
    let xs: Vec<f32> = line.glyphs().iter().map(|placed| placed.x).collect();
    assert!(close(xs[0], 0.0) && close(xs[1], 6.39), "{xs:?}");
    assert!(close(line.width(), 12.39));
    assert!(close(line.ascent(), 10.69) && close(line.descent(), 2.93));

    // Tracking adds space between glyphs, not after the last one.
    let tracked = TextStyle {
        tracking: 0.1,
        ..regular
    };
    line.layout(&fonts, "AV", &tracked);
    assert!(close(line.glyphs()[1].x, 7.39) && close(line.width(), 13.39));

    // Control characters are skipped; missing characters show the missing-glyph box.
    line.layout(&fonts, "A\tV\n", &regular);
    assert_eq!(line.glyphs().len(), 2);
    line.layout(&fonts, "中", &regular);
    assert_eq!(line.glyphs()[0].glyph, GlyphId::NOTDEF);
    assert!(close(line.width(), 6.0));
    assert_eq!(line.glyphs()[0].cluster, 0);

    // Clusters are byte offsets into the text.
    line.layout(&fonts, "éA", &regular);
    assert_eq!(line.glyphs()[1].cluster, 2);
    assert!(close(
        line.centered_baseline(0.0, 20.0),
        10.0 - 6.81 + 10.69
    ));
}

#[test]
fn truncation_keeps_what_fits_and_appends_an_ellipsis() {
    let fonts = Fonts::with_embedded();
    let regular = style(&fonts, 400, 10.0);
    let font = fonts.get(regular.font).unwrap();
    let mut line = TextLine::new();
    // "Outliner" measures 38.71 pixels at 10 pixels per em.
    line.layout_truncated(&fonts, "Outliner", &regular, 38.71);
    assert!(!line.is_truncated() && line.glyphs().len() == 8);
    // At 20 pixels: "O" (7.81) and the ellipsis (7.91).
    line.layout_truncated(&fonts, "Outliner", &regular, 20.0);
    assert!(line.is_truncated());
    let glyphs: Vec<GlyphId> = line.glyphs().iter().map(|placed| placed.glyph).collect();
    assert_eq!(glyphs, [glyph(font, 'O'), glyph(font, '…')]);
    assert!(close(line.width(), 15.72) && close(line.glyphs()[1].x, 7.81));

    // Spaces before the ellipsis go: "ab…", not "ab …".
    let width = |text: &str| {
        let mut measured = TextLine::new();
        measured.layout(&fonts, text, &regular);
        measured.width()
    };
    let room = width("ab ") + width("…") + 0.01;
    assert!(room < width("ab c") + width("…"));
    line.layout_truncated(&fonts, "ab cd", &regular, room);
    let glyphs: Vec<GlyphId> = line.glyphs().iter().map(|placed| placed.glyph).collect();
    assert_eq!(
        glyphs,
        [glyph(font, 'a'), glyph(font, 'b'), glyph(font, '…')]
    );

    // Without room for the ellipsis itself, nothing is shown.
    line.layout_truncated(&fonts, "Outliner", &regular, 5.0);
    assert!(line.is_truncated() && line.is_empty() && line.width() == 0.0);
}

#[test]
fn fonts_select_by_family_and_weight() {
    let fonts = Fonts::with_embedded();
    assert_eq!(fonts.len(), 2);
    let weight = |family: &str, weight| {
        fonts
            .select(family, weight)
            .and_then(|id| fonts.get(id))
            .map(Font::weight)
    };
    assert_eq!(weight("NotoSans", 400), Some(400));
    assert_eq!(weight("Noto Sans", 500), Some(500));
    // Bold falls back to the heaviest lighter weight, light to the lightest heavier one.
    assert_eq!(weight("noto-sans", 700), Some(500));
    assert_eq!(weight("Noto Sans", 300), Some(400));
    assert_eq!(weight("Inter", 400), None);
}

/// The byte range of `tag` in a font file.
fn table(data: &[u8], tag: [u8; 4]) -> std::ops::Range<usize> {
    let be32 = |at: usize| {
        let bytes = data[at..at + 4].try_into().expect("four bytes");
        usize::try_from(u32::from_be_bytes(bytes)).expect("offsets fit in memory")
    };
    let count = usize::from(u16::from_be_bytes([data[4], data[5]]));
    (0..count)
        .map(|index| 12 + 16 * index)
        .find(|record| data[*record..record + 4] == tag)
        .map(|record| be32(record + 8)..be32(record + 8) + be32(record + 12))
        .expect("the embedded font has the table")
}

#[test]
fn broken_fonts_are_refused_with_a_reason() {
    assert_eq!(Font::from_vec(vec![0; 64]).err(), Some(Error::NotAFont));
    assert_eq!(Font::from_vec(Vec::new()).err(), Some(Error::NotAFont));
    let mut cff = embedded::NOTO_SANS_REGULAR.to_vec();
    cff[..4].copy_from_slice(b"OTTO");
    assert!(matches!(
        Font::from_vec(cff).err(),
        Some(Error::Unsupported(_))
    ));

    // A font whose glyph table was renamed.
    let mut renamed = embedded::NOTO_SANS_REGULAR.to_vec();
    let count = u16::from_be_bytes([renamed[4], renamed[5]]) as usize;
    let record = (0..count)
        .map(|index| 12 + 16 * index)
        .find(|record| &renamed[*record..record + 4] == b"glyf")
        .unwrap();
    renamed[record..record + 4].copy_from_slice(b"gly_");
    assert_eq!(
        Font::from_vec(renamed).err(),
        Some(Error::MissingTable(Tag(*b"glyf")))
    );

    let mut bad_magic = embedded::NOTO_SANS_REGULAR.to_vec();
    let head = table(&bad_magic, *b"head");
    bad_magic[head.start + 12] ^= 0xFF;
    assert!(matches!(
        Font::from_vec(bad_magic).err(),
        Some(Error::Malformed { table, .. }) if table == Tag(*b"head")
    ));
}

#[test]
fn self_referencing_composites_are_refused() {
    // Point the first component of "é" (glyph 171) at "é" itself.
    let mut data = embedded::NOTO_SANS_REGULAR.to_vec();
    let loca = table(&data, *b"loca");
    let glyf = table(&data, *b"glyf");
    let at = loca.start + 4 * 171;
    let offset = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
    let component = glyf.start + offset + 10;
    data[component + 2..component + 4].copy_from_slice(&171_u16.to_be_bytes());
    let font = Font::from_vec(data).unwrap();
    let mut outline = Outline::new();
    assert_eq!(outline.load(&font, GlyphId(171)), Err(Error::TooComplex));
    assert!(outline.is_empty());
}

/// A deterministic pseudo-random sequence (xorshift).
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("a small bound");
        usize::try_from(self.next() % bound).expect("below a usize bound")
    }
}

/// Uses every entry point on `font`: none may panic or hang.
fn exercise(font: &Font, rasterizer: &mut Rasterizer, bitmap: &mut GlyphBitmap) {
    let mut outline = Outline::new();
    for c in ['A', 'é', 'o', '€', '…', 'Ж', '\u{FFFF}', '\u{10FFFF}'] {
        let glyph = font.glyph_index(c).unwrap_or(GlyphId::NOTDEF);
        let _ = font.advance(glyph);
        let _ = outline.load(font, glyph);
        let _ = rasterizer.rasterize(font, glyph, 24.0, 0.5, bitmap);
    }
    for index in [0, 1, 100, 171, 4502, 4503, u16::MAX] {
        let _ = rasterizer.rasterize(font, GlyphId(index), 13.0, 0.25, bitmap);
    }
}

#[test]
fn truncated_fonts_never_panic() {
    let data = embedded::NOTO_SANS_REGULAR;
    let mut rasterizer = Rasterizer::new();
    let mut bitmap = GlyphBitmap::default();
    let mut noise = Noise(0x9E37_79B9_7F4A_7C15);
    // Every cut inside the table directory, then cuts anywhere.
    let cuts = (0..400).chain((0..200).map(|_| noise.below(data.len())));
    for cut in cuts {
        if let Ok(font) = Font::from_vec(data[..cut].to_vec()) {
            exercise(&font, &mut rasterizer, &mut bitmap);
        }
    }
}

#[test]
fn corrupted_fonts_never_panic() {
    let data = embedded::NOTO_SANS_REGULAR;
    let tables = [
        b"cmap", b"loca", b"glyf", b"hmtx", b"head", b"hhea", b"maxp",
    ]
    .map(|tag| table(data, *tag));
    let mut rasterizer = Rasterizer::new();
    let mut bitmap = GlyphBitmap::default();
    let mut noise = Noise(0x2545_F491_4F6C_DD1D);
    for round in 0..300 {
        let mut corrupted = data.to_vec();
        // Hit the directory in some rounds and the tables in the others.
        for _ in 0..=noise.below(8) {
            let at = if round % 4 == 0 {
                noise.below(12 + 16 * 16)
            } else {
                let range = &tables[noise.below(tables.len())];
                range.start + noise.below(range.len().max(1))
            };
            corrupted[at] = (noise.next() & 0xFF) as u8;
        }
        if let Ok(font) = Font::from_vec(corrupted) {
            exercise(&font, &mut rasterizer, &mut bitmap);
        }
    }
}

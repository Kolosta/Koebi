//! From an SVG file to an [`Icon`]: the elements walked with the styles they inherit, shapes
//! turned into outlines, strokes into fills, transforms applied, gradients resolved, colors
//! grouped into zones, and everything mapped into the icon's unit square, its view box kept
//! whole (an icon drawn against one side keeps the space on the other).
//!
//! What the subset holds: `svg` (the root), `g`, `defs`, `path`, `rect`, `circle`, `ellipse`,
//! `line`, `polyline`, `polygon`, `linearGradient`, `radialGradient` and their `stop`s, `title`,
//! `desc` and `metadata` (skipped); the attributes `fill`, `fill-opacity`, `fill-rule`, `stroke`,
//! `stroke-width`, `stroke-opacity`, `stroke-linecap`, `stroke-linejoin`, `stroke-miterlimit`,
//! `opacity`, `color`, `transform`, `class`, `display`, `visibility`, and the same in `style`.
//! Anything that would draw otherwise (`text`, `image`, `use`, masks, clips, filters, patterns,
//! dashes, CSS) is an error naming the line: an icon never draws differently from its file.

use std::collections::HashMap;

use crate::color::{PaintValue, Rgba, parse_color, parse_paint};
use crate::error::{Error, Result};
use crate::geometry::{Affine, Outline, Point};
use crate::model::{FillRule, Icon, Layer, Paint, Role, Stop, Zone};
use crate::path::{ellipse, numbers, parse_path, polyline, rect};
use crate::stroke::{Cap, Join, StrokeStyle};
use crate::xml::{self, Element};

/// How far curves may stray from the outline, in shares of the icon's side.
const CURVE_TOLERANCE: f32 = 1e-4;
/// How far a stroke's flattened outline may stray, in shares of the icon's side.
const STROKE_TOLERANCE: f32 = 2.5e-4;
/// The most color zones an icon has: what the renderer recolors per instance.
pub const MAX_ZONES: usize = 8;

/// Elements that would draw but are not supported, and how to do without them.
const UNSUPPORTED: [(&str, &str); 16] = [
    ("text", "convert the text to paths"),
    ("tspan", "convert the text to paths"),
    ("textPath", "convert the text to paths"),
    ("image", "icons are vector only"),
    ("use", "copy the element it uses in place"),
    ("symbol", "copy its content in place"),
    ("mask", "flatten the masked content into paths"),
    ("clipPath", "apply the clip to the paths (intersect them)"),
    ("filter", "remove the filter"),
    ("pattern", "use a plain color or a gradient"),
    ("marker", "draw the markers as paths"),
    ("foreignObject", "icons are SVG only"),
    ("style", "write the styles as attributes or `style`"),
    ("switch", "keep one alternative"),
    (
        "svg",
        "nested `svg` elements are not supported: use a `g` with a transform",
    ),
    ("a", "remove the link and keep what it holds"),
];

/// Compiles the SVG file `text`.
///
/// # Errors
///
/// The file is not well-formed XML, its root is not `svg`, it has no view box, it uses what the
/// subset does not hold (named, with how to do without), has more than [`MAX_ZONES`] colors, or
/// draws nothing. The error names the line.
pub fn compile(text: &str) -> Result<Icon> {
    let root = xml::parse(text)?;
    if root.name != "svg" {
        return Err(Error::at(root.line, "the root element must be `svg`"));
    }
    let (view_origin, view_size) = view_box(&root)?;
    // The view box into the unit square, centered when it is not square.
    let side = view_size.x.max(view_size.y);
    let offset = Point::new((side - view_size.x) * 0.5, (side - view_size.y) * 0.5);
    let to_square = Affine([
        1.0 / side,
        0.0,
        0.0,
        1.0 / side,
        (offset.x - view_origin.x) / side,
        (offset.y - view_origin.y) / side,
    ]);
    let mut compiler = Compiler {
        gradients: HashMap::new(),
        layers: Vec::new(),
        zones: Vec::new(),
        to_square,
    };
    compiler.collect_gradients(&root)?;
    let style = Style::from_element(&Style::ROOT, &root)?;
    for child in &root.children {
        compiler.walk(child, &style, Affine::IDENTITY)?;
    }
    if compiler.layers.is_empty() {
        return Err(Error::at(0, "the icon draws nothing"));
    }
    let (layers, zones) = sort_zones(compiler.layers, &compiler.zones);
    Ok(Icon {
        view_size: [view_size.x, view_size.y],
        layers,
        zones,
    })
}

/// The view box: `viewBox`, or the width and height without one.
fn view_box(root: &Element) -> Result<(Point, Point)> {
    if let Some(text) = root.attribute("viewBox") {
        let values = numbers(text)
            .filter(|values| values.len() == 4)
            .ok_or_else(|| Error::at(root.line, format!("malformed viewBox {text:?}")))?;
        if values[2] <= 0.0 || values[3] <= 0.0 {
            return Err(Error::at(root.line, "the viewBox has no area"));
        }
        return Ok((
            Point::new(values[0], values[1]),
            Point::new(values[2], values[3]),
        ));
    }
    let length = |name: &str| {
        root.attribute(name)
            .and_then(|text| text.trim().trim_end_matches("px").parse::<f32>().ok())
            .filter(|value| *value > 0.0)
    };
    match (length("width"), length("height")) {
        (Some(width), Some(height)) => Ok((Point::default(), Point::new(width, height))),
        _ => Err(Error::at(
            root.line,
            "the icon needs a viewBox (or a width and a height in user units)",
        )),
    }
}

/// The styles an element inherits, and those of its own.
#[derive(Clone, Debug)]
struct Style {
    fill: PaintValue,
    fill_opacity: f32,
    fill_rule: FillRule,
    stroke_paint: PaintValue,
    stroke_opacity: f32,
    stroke: StrokeStyleOptions,
    /// The product of the `opacity` of the element and of its ancestors.
    opacity: f32,
    color: Rgba,
    role: Option<Role>,
    hidden: bool,
}

#[derive(Clone, Copy, Debug)]
struct StrokeStyleOptions {
    width: f32,
    cap: Cap,
    join: Join,
    miter_limit: f32,
}

impl Style {
    const ROOT: Self = Self {
        fill: PaintValue::Color(Rgba::BLACK),
        fill_opacity: 1.0,
        fill_rule: FillRule::NonZero,
        stroke_paint: PaintValue::None,
        stroke_opacity: 1.0,
        stroke: StrokeStyleOptions {
            width: 1.0,
            cap: Cap::Butt,
            join: Join::Miter,
            miter_limit: 4.0,
        },
        opacity: 1.0,
        color: Rgba::BLACK,
        role: None,
        hidden: false,
    };

    /// The style of `element` under `parent`'s.
    fn from_element(parent: &Self, element: &Element) -> Result<Self> {
        let mut style = parent.clone();
        // `opacity` does not inherit; it multiplies down the tree.
        let mut properties: Vec<(&str, &str)> = element
            .attributes
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        if let Some(inline) = element.attribute("style") {
            for declaration in inline.split(';') {
                if let Some((key, value)) = declaration.split_once(':') {
                    properties.push((key.trim(), value.trim()));
                }
            }
        }
        let line = element.line;
        let number = |value: &str, name: &str| {
            value
                .trim()
                .trim_end_matches("px")
                .parse::<f32>()
                .map_err(|_| Error::at(line, format!("malformed {name} {value:?}")))
        };
        let opacity = |value: &str, name: &str| -> Result<f32> {
            let value = value.trim();
            let parsed = match value.strip_suffix('%') {
                Some(percent) => number(percent, name)? / 100.0,
                None => number(value, name)?,
            };
            Ok(parsed.clamp(0.0, 1.0))
        };
        for (key, value) in properties {
            match key {
                "fill" => style.fill = paint(value, line)?,
                "fill-opacity" => style.fill_opacity = opacity(value, key)?,
                "fill-rule" => {
                    style.fill_rule = match value.trim() {
                        "evenodd" => FillRule::EvenOdd,
                        "nonzero" => FillRule::NonZero,
                        other => {
                            return Err(Error::at(line, format!("unknown fill-rule {other:?}")));
                        }
                    }
                }
                "stroke" => style.stroke_paint = paint(value, line)?,
                "stroke-opacity" => style.stroke_opacity = opacity(value, key)?,
                "stroke-width" => style.stroke.width = number(value, key)?.max(0.0),
                "stroke-linecap" => {
                    style.stroke.cap = match value.trim() {
                        "butt" => Cap::Butt,
                        "round" => Cap::Round,
                        "square" => Cap::Square,
                        other => {
                            return Err(Error::at(line, format!("unknown linecap {other:?}")));
                        }
                    }
                }
                "stroke-linejoin" => {
                    style.stroke.join = match value.trim() {
                        "miter" | "miter-clip" | "arcs" => Join::Miter,
                        "round" => Join::Round,
                        "bevel" => Join::Bevel,
                        other => {
                            return Err(Error::at(line, format!("unknown linejoin {other:?}")));
                        }
                    }
                }
                "stroke-miterlimit" => style.stroke.miter_limit = number(value, key)?.max(1.0),
                "opacity" => style.opacity = parent.opacity * opacity(value, key)?,
                "color" => {
                    style.color = parse_color(value)
                        .ok_or_else(|| Error::at(line, format!("malformed color {value:?}")))?;
                }
                "class" => {
                    if let Some(role) = Role::of_class(value) {
                        style.role = Some(role);
                    }
                }
                "display" if value.trim() == "none" => style.hidden = true,
                "visibility" if matches!(value.trim(), "hidden" | "collapse") => {
                    style.hidden = true;
                }
                "stroke-dasharray" if value.trim() != "none" => {
                    return Err(Error::at(
                        line,
                        "dashed strokes are not supported: draw the dashes as paths",
                    ));
                }
                "clip-path" | "mask" | "filter" if value.trim() != "none" => {
                    return Err(Error::at(
                        line,
                        format!("`{key}` is not supported: apply it to the paths themselves"),
                    ));
                }
                _ => {}
            }
        }
        Ok(style)
    }
}

fn paint(value: &str, line: usize) -> Result<PaintValue> {
    parse_paint(value).ok_or_else(|| Error::at(line, format!("malformed paint {value:?}")))
}

/// A gradient as written, its `href` resolved later.
#[derive(Clone, Debug)]
struct GradientDef {
    radial: bool,
    attributes: Vec<(String, String)>,
    stops: Vec<Element>,
    href: Option<String>,
    line: usize,
}

struct Compiler {
    gradients: HashMap<String, GradientDef>,
    layers: Vec<Layer>,
    /// Zones by first use: their opaque color, role, and the key that finds them.
    zones: Vec<(Rgba, Role, [u8; 3])>,
    to_square: Affine,
}

impl Compiler {
    fn collect_gradients(&mut self, element: &Element) -> Result<()> {
        for child in &element.children {
            if matches!(child.name.as_str(), "linearGradient" | "radialGradient") {
                let id = child
                    .attribute("id")
                    .ok_or_else(|| Error::at(child.line, "a gradient needs an id"))?;
                let href = child
                    .attribute("href")
                    .or_else(|| child.attribute("xlink:href"))
                    .and_then(|href| href.strip_prefix('#'))
                    .map(str::to_owned);
                for stop in &child.children {
                    if stop.name != "stop" {
                        return Err(Error::at(
                            stop.line,
                            format!("a gradient holds stops only, not `{}`", stop.name),
                        ));
                    }
                }
                self.gradients.insert(
                    id.to_owned(),
                    GradientDef {
                        radial: child.name == "radialGradient",
                        attributes: child.attributes.clone(),
                        stops: child.children.clone(),
                        href,
                        line: child.line,
                    },
                );
            } else {
                self.collect_gradients(child)?;
            }
        }
        Ok(())
    }

    fn walk(&mut self, element: &Element, parent: &Style, transform: Affine) -> Result<()> {
        let name = element.name.as_str();
        if let Some((_, hint)) = UNSUPPORTED
            .iter()
            .find(|(unsupported, _)| *unsupported == name)
        {
            return Err(Error::at(
                element.line,
                format!("`{name}` is not supported: {hint}"),
            ));
        }
        if matches!(
            name,
            "defs" | "title" | "desc" | "metadata" | "linearGradient" | "radialGradient"
        ) || name.starts_with("sodipodi:")
            || name.starts_with("inkscape:")
        {
            return Ok(());
        }
        let style = Style::from_element(parent, element)?;
        if style.hidden {
            return Ok(());
        }
        let transform = match element.attribute("transform") {
            Some(text) => transform.then(crate::geometry::parse_transform(text, element.line)?),
            None => transform,
        };
        if element.has_text && name != "g" {
            return Err(Error::at(
                element.line,
                format!("text inside `{name}` is not supported"),
            ));
        }
        let outline = match name {
            "g" => {
                for child in &element.children {
                    self.walk(child, &style, transform)?;
                }
                return Ok(());
            }
            "path" => match element.attribute("d") {
                Some(data) => parse_path(data, element.line)?,
                None => return Ok(()),
            },
            "rect" => {
                let width = length(element, "width")?.unwrap_or(0.0);
                let height = length(element, "height")?.unwrap_or(0.0);
                if width <= 0.0 || height <= 0.0 {
                    return Ok(());
                }
                rect(
                    length(element, "x")?.unwrap_or(0.0),
                    length(element, "y")?.unwrap_or(0.0),
                    width,
                    height,
                    length(element, "rx")?,
                    length(element, "ry")?,
                )
            }
            "circle" => {
                let r = length(element, "r")?.unwrap_or(0.0);
                if r <= 0.0 {
                    return Ok(());
                }
                ellipse(
                    length(element, "cx")?.unwrap_or(0.0),
                    length(element, "cy")?.unwrap_or(0.0),
                    r,
                    r,
                )
            }
            "ellipse" => {
                let rx = length(element, "rx")?.unwrap_or(0.0);
                let ry = length(element, "ry")?.unwrap_or(0.0);
                if rx <= 0.0 || ry <= 0.0 {
                    return Ok(());
                }
                ellipse(
                    length(element, "cx")?.unwrap_or(0.0),
                    length(element, "cy")?.unwrap_or(0.0),
                    rx,
                    ry,
                )
            }
            "line" => {
                let at = |name: &str| -> Result<f32> { Ok(length(element, name)?.unwrap_or(0.0)) };
                polyline(
                    &[at("x1")?, at("y1")?, at("x2")?, at("y2")?],
                    false,
                    element.line,
                )?
            }
            "polyline" | "polygon" => {
                let points = element
                    .attribute("points")
                    .and_then(numbers)
                    .ok_or_else(|| Error::at(element.line, "malformed points"))?;
                polyline(&points, name == "polygon", element.line)?
            }
            other => {
                return Err(Error::at(
                    element.line,
                    format!("`{other}` is not supported by Koebi's icons"),
                ));
            }
        };
        self.shape(element, &style, transform, &outline)
    }

    /// The fill and the stroke of a shape.
    fn shape(
        &mut self,
        element: &Element,
        style: &Style,
        transform: Affine,
        outline: &Outline,
    ) -> Result<()> {
        let to_square = self.to_square.then(transform);
        let bounds = outline.bounds();
        if !matches!(style.fill, PaintValue::None) && element.name != "line" {
            let paint = self.paint(
                &style.fill,
                style,
                style.fill_opacity,
                transform,
                bounds,
                element.line,
            )?;
            let curves = outline.transformed(to_square).fill_quads(CURVE_TOLERANCE);
            self.push(curves, style.fill_rule, paint);
        }
        if !matches!(style.stroke_paint, PaintValue::None) && style.stroke.width > 0.0 {
            let paint = self.paint(
                &style.stroke_paint,
                style,
                style.stroke_opacity,
                transform,
                bounds,
                element.line,
            )?;
            let width = style.stroke.width * to_square.scale_factor();
            let loops = crate::stroke::outline(
                &outline.transformed(to_square),
                StrokeStyle {
                    width,
                    cap: style.stroke.cap,
                    join: style.stroke.join,
                    miter_limit: style.stroke.miter_limit,
                },
                STROKE_TOLERANCE,
            );
            let curves = loops.into_iter().flatten().collect();
            self.push(curves, FillRule::NonZero, paint);
        }
        Ok(())
    }

    fn push(&mut self, curves: Vec<[Point; 3]>, rule: FillRule, paint: Paint) {
        let visible = match &paint {
            Paint::Solid(stop) => stop.color.0[3] > 0.0,
            Paint::Gradient { stops, .. } => stops.iter().any(|stop| stop.color.0[3] > 0.0),
        };
        if visible && !curves.is_empty() {
            let mut monotonic = Vec::with_capacity(curves.len());
            for curve in curves {
                crate::geometry::monotonic(curve, &mut monotonic);
            }
            self.layers.push(Layer {
                curves: monotonic,
                rule,
                paint,
            });
        }
    }

    /// The zone of `color` with `role`: a new one for a new color, the strongest role kept.
    fn zone(&mut self, color: Rgba, role: Option<Role>, line: usize) -> Result<u8> {
        let [r, g, b, _] = color.to_rgba8();
        let key = [r, g, b];
        let role = role.unwrap_or_default();
        if let Some(index) = self.zones.iter().position(|zone| zone.2 == key) {
            let kept = &mut self.zones[index].1;
            *kept = (*kept).min(role);
            #[allow(
                clippy::cast_possible_truncation,
                reason = "fewer than MAX_ZONES zones"
            )]
            return Ok(index as u8);
        }
        if self.zones.len() >= MAX_ZONES {
            return Err(Error::at(
                line,
                format!("more than {MAX_ZONES} colors: icons recolor by zone, {MAX_ZONES} at most"),
            ));
        }
        let [cr, cg, cb, _] = color.0;
        self.zones.push((Rgba([cr, cg, cb, 1.0]), role, key));
        #[allow(
            clippy::cast_possible_truncation,
            reason = "fewer than MAX_ZONES zones"
        )]
        Ok((self.zones.len() - 1) as u8)
    }

    /// The paint of `value` for a shape with `style`, its opacities folded in.
    #[allow(clippy::too_many_arguments, reason = "what a paint depends on")]
    fn paint(
        &mut self,
        value: &PaintValue,
        style: &Style,
        opacity: f32,
        transform: Affine,
        bounds: Option<(Point, Point)>,
        line: usize,
    ) -> Result<Paint> {
        let opacity = opacity * style.opacity;
        match value {
            PaintValue::None => Err(Error::at(line, "nothing to paint with")),
            PaintValue::Color(_) | PaintValue::Current => {
                let color = match value {
                    PaintValue::Color(color) => *color,
                    _ => style.color,
                };
                let zone = self.zone(color, style.role, line)?;
                Ok(Paint::Solid(Stop {
                    offset: 0.0,
                    color: color.faded(opacity),
                    zone,
                }))
            }
            PaintValue::Url(id) => self.gradient(id, style, opacity, transform, bounds, line),
        }
    }

    #[allow(clippy::too_many_arguments, reason = "what a gradient depends on")]
    fn gradient(
        &mut self,
        id: &str,
        style: &Style,
        opacity: f32,
        transform: Affine,
        bounds: Option<(Point, Point)>,
        line: usize,
    ) -> Result<Paint> {
        let def = self
            .gradients
            .get(id)
            .cloned()
            .ok_or_else(|| Error::at(line, format!("no gradient with the id {id:?}")))?;
        // Attributes and stops it inherits through `href`, eight gradients deep at most.
        let mut chain = vec![def.clone()];
        let mut next = def.href.clone();
        while let Some(href) = next.take() {
            if chain.len() > 8 {
                return Err(Error::at(
                    def.line,
                    "gradients refer to each other in a loop",
                ));
            }
            let parent =
                self.gradients.get(&href).cloned().ok_or_else(|| {
                    Error::at(def.line, format!("no gradient with the id {href:?}"))
                })?;
            next.clone_from(&parent.href);
            chain.push(parent);
        }
        let attribute = |name: &str| {
            chain.iter().find_map(|def| {
                def.attributes
                    .iter()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value.clone())
            })
        };
        let stops_from = chain
            .iter()
            .find(|def| !def.stops.is_empty())
            .ok_or_else(|| Error::at(def.line, "the gradient has no stops"))?;
        if attribute("spreadMethod").is_some_and(|spread| spread.trim() != "pad") {
            return Err(Error::at(def.line, "only padded gradients are supported"));
        }
        let user_space =
            attribute("gradientUnits").is_some_and(|units| units.trim() == "userSpaceOnUse");
        let coordinate = |name: &str, default: f32| -> Result<f32> {
            match attribute(name) {
                Some(text) => {
                    let text = text.trim();
                    match text.strip_suffix('%') {
                        Some(percent) => percent
                            .parse::<f32>()
                            .map(|value| value / 100.0)
                            .map_err(|_| Error::at(def.line, format!("malformed {name}"))),
                        None => text
                            .parse::<f32>()
                            .map_err(|_| Error::at(def.line, format!("malformed {name}"))),
                    }
                }
                None => Ok(default),
            }
        };
        let space = if user_space {
            Affine::IDENTITY
        } else {
            let (low, high) = bounds.unwrap_or_default();
            let size = high.sub(low);
            Affine([size.x.max(1e-6), 0.0, 0.0, size.y.max(1e-6), low.x, low.y])
        };
        let gradient_transform = match attribute("gradientTransform") {
            Some(text) => crate::geometry::parse_transform(&text, def.line)?,
            None => Affine::IDENTITY,
        };
        let local = if def.radial {
            let (cx, cy) = (coordinate("cx", 0.5)?, coordinate("cy", 0.5)?);
            let r = coordinate("r", 0.5)?;
            let fx = coordinate("fx", cx)?;
            let fy = coordinate("fy", cy)?;
            if (fx - cx).abs() > 1e-6 || (fy - cy).abs() > 1e-6 {
                return Err(Error::at(
                    def.line,
                    "radial gradients with a focal point apart from the center are not supported",
                ));
            }
            Affine([r, 0.0, 0.0, r, cx, cy])
        } else {
            let (x1, y1) = (coordinate("x1", 0.0)?, coordinate("y1", 0.0)?);
            let (x2, y2) = (coordinate("x2", 1.0)?, coordinate("y2", 0.0)?);
            let (dx, dy) = (x2 - x1, y2 - y1);
            Affine([dx, dy, -dy, dx, x1, y1])
        };
        let to_icon = self
            .to_square
            .then(transform)
            .then(space)
            .then(gradient_transform)
            .then(local);
        let to_gradient = to_icon
            .inverse()
            .ok_or_else(|| Error::at(def.line, "the gradient has no extent"))?;
        let mut stops = Vec::new();
        let mut last = 0.0_f32;
        for stop in &stops_from.stops {
            let stop_style = Style::from_element(style, stop)?;
            let mut color = Rgba::BLACK;
            let mut stop_opacity = 1.0;
            let mut properties: Vec<(String, String)> = stop.attributes.clone();
            if let Some(inline) = stop.attribute("style") {
                for declaration in inline.split(';') {
                    if let Some((key, value)) = declaration.split_once(':') {
                        properties.push((key.trim().to_owned(), value.trim().to_owned()));
                    }
                }
            }
            for (key, value) in &properties {
                match key.as_str() {
                    "stop-color" => {
                        color = if value.trim() == "currentColor" {
                            style.color
                        } else {
                            parse_color(value).ok_or_else(|| {
                                Error::at(stop.line, format!("malformed stop-color {value:?}"))
                            })?
                        };
                    }
                    "stop-opacity" => {
                        stop_opacity = value.trim().parse::<f32>().map_err(|_| {
                            Error::at(stop.line, format!("malformed stop-opacity {value:?}"))
                        })?;
                    }
                    _ => {}
                }
            }
            let offset = match stop.attribute("offset") {
                Some(text) => {
                    let text = text.trim();
                    match text.strip_suffix('%') {
                        Some(percent) => percent.parse::<f32>().map(|value| value / 100.0),
                        None => text.parse::<f32>(),
                    }
                    .map_err(|_| Error::at(stop.line, "malformed offset"))?
                }
                None => 0.0,
            };
            // Offsets never go back.
            let offset = offset.clamp(0.0, 1.0).max(last);
            last = offset;
            let zone = self.zone(color, stop_style.role, stop.line)?;
            stops.push(Stop {
                offset,
                color: color.faded(stop_opacity.clamp(0.0, 1.0) * opacity),
                zone,
            });
        }
        if stops.len() == 1 {
            return Ok(Paint::Solid(Stop {
                offset: 0.0,
                ..stops[0]
            }));
        }
        Ok(Paint::Gradient {
            radial: def.radial,
            to_gradient: to_gradient.0,
            stops,
        })
    }
}

/// The length `name` of `element`, in user units.
fn length(element: &Element, name: &str) -> Result<Option<f32>> {
    element
        .attribute(name)
        .map(|text| {
            let text = text.trim();
            if text.ends_with('%') {
                return Err(Error::at(
                    element.line,
                    format!("percentages are not supported for {name}"),
                ));
            }
            text.trim_end_matches("px")
                .parse::<f32>()
                .map_err(|_| Error::at(element.line, format!("malformed {name} {text:?}")))
        })
        .transpose()
}

/// The zones in order of role, then of first use; the layers' stops follow.
fn sort_zones(mut layers: Vec<Layer>, zones: &[(Rgba, Role, [u8; 3])]) -> (Vec<Layer>, Vec<Zone>) {
    let mut order: Vec<usize> = (0..zones.len()).collect();
    order.sort_by_key(|&index| (zones[index].1, index));
    let mut new_index = vec![0_u8; zones.len()];
    for (position, &old) in order.iter().enumerate() {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "fewer than MAX_ZONES zones"
        )]
        {
            new_index[old] = position as u8;
        }
    }
    let renumber = |stop: &mut Stop| stop.zone = new_index[usize::from(stop.zone)];
    for layer in &mut layers {
        match &mut layer.paint {
            Paint::Solid(stop) => renumber(stop),
            Paint::Gradient { stops, .. } => stops.iter_mut().for_each(renumber),
        }
    }
    let zones = order
        .iter()
        .map(|&index| Zone {
            color: zones[index].0,
            role: zones[index].1,
        })
        .collect();
    (layers, zones)
}

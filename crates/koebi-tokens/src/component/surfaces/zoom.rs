//! Zoomable spaces: a node graph, a preview, what a component shows scaled (ADR 0030).

use koebi_design::define::*;

koebi_design::tokens! {
    group ZOOM: Component, start = crate::component::CURSOR::END;

    /// Zoomed content: the smallest size on screen at which text is still read. Components
    /// drawn smaller in a zoomed-out space drop their labels and details.
    ZOOM_READABLE_SIZE_DEFAULT = "zoom.readable-size.default": Dimension = px(7.0),
        range(1.0, 32.0);
}

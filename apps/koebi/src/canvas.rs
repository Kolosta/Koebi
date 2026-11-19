//! The document drawn for the viewports of a window: each viewport asks for its view while the
//! interface is built ([`CanvasViews`]), the window's GPU draws every view into a target of its
//! own and shows it as an image of the window's renderer ([`CanvasTargets`]).
//!
//! A view seen for the first time shows from the next frame, once its target exists; a view
//! that changes size keeps its image, its target made anew; a view no viewport asked for in a
//! frame gives back its target and its image.

use koebi_canvas::{Canvas, View, ViewTarget};
use koebi_doc::eval::Program;
use koebi_gpu::{CommandEncoder, Device};
use koebi_render::{ImageContent, ImageId, Renderer2D};
use koebi_workspace::EditorId;

/// The views of the document the viewports of one window ask for, frame after frame. Its
/// storage stays.
#[derive(Clone, Debug, Default)]
pub(crate) struct CanvasViews {
    views: Vec<CanvasView>,
}

/// One viewport's view.
#[derive(Clone, Copy, Debug, PartialEq)]
struct CanvasView {
    editor: EditorId,
    view: View,
    /// The image showing it, once the GPU made one.
    image: Option<ImageId>,
    /// Whether a viewport asked for it this frame.
    wanted: bool,
}

impl CanvasViews {
    /// Starts a frame: no view is asked for yet.
    pub(crate) fn begin_frame(&mut self) {
        for view in &mut self.views {
            view.wanted = false;
        }
    }

    /// Asks for `editor`'s view `view`; returns the image showing it, once there is one.
    pub(crate) fn show(&mut self, editor: EditorId, view: View) -> Option<ImageId> {
        if let Some(known) = self.views.iter_mut().find(|known| known.editor == editor) {
            known.view = view;
            known.wanted = true;
            return known.image;
        }
        self.views.push(CanvasView {
            editor,
            view,
            image: None,
            wanted: true,
        });
        None
    }

    /// Whether a viewport asked for a view this frame.
    pub(crate) fn any_wanted(&self) -> bool {
        self.views.iter().any(|view| view.wanted)
    }

    /// The views asked for this frame.
    fn wanted(&self) -> impl Iterator<Item = (EditorId, View)> + '_ {
        self.views
            .iter()
            .filter(|view| view.wanted)
            .map(|view| (view.editor, view.view))
    }
}

/// The targets a window's views are drawn into, each shown as an image of its renderer.
#[derive(Default)]
pub(crate) struct CanvasTargets {
    targets: Vec<Shown>,
}

/// A view's target and the image showing it.
struct Shown {
    editor: EditorId,
    target: ViewTarget,
    image: ImageId,
}

impl std::fmt::Debug for CanvasTargets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanvasTargets")
            .field("targets", &self.targets.len())
            .finish()
    }
}

impl CanvasTargets {
    /// No target yet.
    pub(crate) const fn new() -> Self {
        Self {
            targets: Vec::new(),
        }
    }

    /// Forgets every target, as when the renderer showing them is made anew: its images are
    /// gone, and the views get new ones.
    pub(crate) fn reset(&mut self, views: &mut CanvasViews) {
        self.targets.clear();
        for view in &mut views.views {
            view.image = None;
        }
    }

    /// Draws `program` into the target of every view asked for this frame, made or resized as
    /// it must be, and gives back the targets of the views no longer asked for. Returns `true`
    /// when a view got its first image: the frame shows it from the next one.
    ///
    /// # Errors
    ///
    /// When a target or an image cannot be made, or the canvas cannot draw.
    pub(crate) fn render(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder<'_>,
        canvas: &mut Canvas,
        renderer: &mut Renderer2D,
        views: &mut CanvasViews,
        program: Option<&Program>,
    ) -> koebi_gpu::Result<bool> {
        // The views no longer asked for, or with nothing to draw, give back their targets.
        self.targets.retain(|shown| {
            let kept =
                program.is_some() && views.wanted().any(|(editor, _)| editor == shown.editor);
            if !kept {
                renderer.remove_image(shown.image);
            }
            kept
        });
        views
            .views
            .retain(|view| view.wanted && (program.is_some() || view.image.is_none()));
        for view in &mut views.views {
            if program.is_none() {
                view.image = None;
            }
        }
        let Some(program) = program else {
            return Ok(false);
        };
        let mut appeared = false;
        for index in 0..views.views.len() {
            let CanvasView {
                editor,
                view,
                image,
                ..
            } = views.views[index];
            let at = self.targets.iter().position(|shown| shown.editor == editor);
            let shown = match at {
                Some(at) if self.targets[at].target.extent() == view.extent => {
                    &mut self.targets[at]
                }
                Some(at) => {
                    // The same image, showing a target of the new size.
                    let target = ViewTarget::new(device, view.extent)?;
                    let shown = &mut self.targets[at];
                    renderer.replace_image(shown.image, target.texture(), ImageContent::Linear)?;
                    shown.target = target;
                    shown
                }
                None => {
                    let target = ViewTarget::new(device, view.extent)?;
                    let image = renderer.add_image(target.texture(), ImageContent::Linear)?;
                    self.targets.push(Shown {
                        editor,
                        target,
                        image,
                    });
                    self.targets.last_mut().expect("just pushed")
                }
            };
            if image != Some(shown.image) {
                views.views[index].image = Some(shown.image);
                appeared = true;
            }
            canvas.render(encoder, &mut shown.target, program, &view)?;
        }
        Ok(appeared)
    }
}

#[cfg(test)]
mod tests {
    use koebi_core::Extent;
    use koebi_geom::Point;

    use super::*;

    fn view(width: u32) -> View {
        View {
            center: Point::ORIGIN,
            zoom: 1.0,
            extent: Extent::new(width, 10),
        }
    }

    #[test]
    fn views_are_asked_for_frame_by_frame() {
        let mut views = CanvasViews::default();
        let editor = EditorId::from_saved(3);
        views.begin_frame();
        assert_eq!(views.show(editor, view(10)), None);
        assert_eq!(views.wanted().count(), 1);
        views.begin_frame();
        assert_eq!(views.wanted().count(), 0);
        views.show(editor, view(20));
        views.show(EditorId::from_saved(4), view(30));
        let wanted: Vec<_> = views.wanted().collect();
        assert_eq!(
            wanted,
            [(editor, view(20)), (EditorId::from_saved(4), view(30))]
        );
    }
}

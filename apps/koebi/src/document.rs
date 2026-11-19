//! The document every editor shows and edits, and its evaluation: one [`Evaluator`] turns the
//! graph into the programs the viewports draw, again only where the graph changed.

use std::sync::Arc;

use koebi_doc::Document;
use koebi_doc::eval::{Evaluator, Program};
use koebi_doc::ops;
use koebi_geom::{Point, Rect};

/// The document open in the application, and what draws it.
pub(crate) struct DocumentState {
    document: Document,
    evaluator: Evaluator,
}

impl std::fmt::Debug for DocumentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentState").finish_non_exhaustive()
    }
}

impl DocumentState {
    /// A new document: one A4 landscape page at the origin, nothing to undo.
    pub(crate) fn new() -> Self {
        let document = match ops::new_document() {
            Ok((document, _)) => document,
            Err(error) => {
                // An empty document always takes a page: this is a defect, shown as one.
                koebi_core::error!("a new document has no page: {error:?}");
                Document::new()
            }
        };
        Self {
            document,
            evaluator: Evaluator::new(),
        }
    }

    /// The program of the first page, which the viewports show: evaluated again when the
    /// document changed, the same one otherwise.
    pub(crate) fn program(&mut self) -> Option<Arc<Program>> {
        self.evaluator.programs(&self.document).first().cloned()
    }

    /// The first page, in document units: where framing the document looks.
    pub(crate) fn page(&mut self) -> Option<Rect> {
        let program = self.evaluator.programs(&self.document).first()?;
        let size = program.size();
        Some(Rect::new(Point::ORIGIN, Point::new(size.x, size.y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_document_shows_its_a4_page_and_keeps_its_program() {
        let mut state = DocumentState::new();
        assert_eq!(
            state.page(),
            Some(Rect::new(Point::ORIGIN, Point::new(297.0, 210.0)))
        );
        let first = state.program().expect("a program");
        let again = state.program().expect("a program");
        // Unchanged, the document's program is not made again.
        assert!(Arc::ptr_eq(&first, &again));
    }
}

//! [`WidgetId`]: the identity of a widget across frames.

/// Identifies a widget across frames, so that a press or a drag survives redraws.
///
/// Build ids from names, and derive the ids of repeated widgets with [`WidgetId::with`]:
///
/// ```
/// # use koebi_ui::WidgetId;
/// const SWATCHES: WidgetId = WidgetId::new("swatches");
/// assert_ne!(SWATCHES.with(0), SWATCHES.with(1));
/// assert_eq!(WidgetId::new("swatches"), SWATCHES);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(u64);

/// FNV-1a, 64 bits: fast, `const`, and good enough to tell a few thousand widgets apart.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

impl WidgetId {
    /// The id of the widget named `name`.
    pub const fn new(name: &str) -> Self {
        Self(fnv1a(FNV_OFFSET_BASIS, name.as_bytes()))
    }

    /// The id of the `index`-th child of this widget: rows, grid cells, parts of a widget.
    #[must_use]
    pub const fn with(self, index: u64) -> Self {
        Self(fnv1a(self.0, &index.to_le_bytes()))
    }
}

const fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        index += 1;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_and_distinct() {
        assert_eq!(WidgetId::new("apply"), WidgetId::new("apply"));
        assert_ne!(WidgetId::new("apply"), WidgetId::new("cancel"));
        // The FNV-1a reference value of the empty input is the offset basis.
        assert_eq!(WidgetId::new(""), WidgetId(FNV_OFFSET_BASIS));
    }

    #[test]
    fn children_differ_from_each_other_and_from_their_parent() {
        let parent = WidgetId::new("list");
        let children: Vec<_> = (0..100).map(|index| parent.with(index)).collect();
        for (index, child) in children.iter().enumerate() {
            assert_ne!(*child, parent);
            assert!(!children[..index].contains(child));
        }
        assert_eq!(parent.with(3), WidgetId::new("list").with(3));
    }
}

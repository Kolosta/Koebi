//! Bounds-checked big-endian reads: fonts are untrusted input.
//!
//! Every read returns `None` past the end of the data instead of panicking; table parsers turn
//! `None` into [`crate::Error::Malformed`] with their table's tag.

use std::fmt;

/// A four-letter table tag (`head`, `glyf`...).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    pub(crate) const CMAP: Self = Self(*b"cmap");
    pub(crate) const GLYF: Self = Self(*b"glyf");
    pub(crate) const HEAD: Self = Self(*b"head");
    pub(crate) const HHEA: Self = Self(*b"hhea");
    pub(crate) const HMTX: Self = Self(*b"hmtx");
    pub(crate) const LOCA: Self = Self(*b"loca");
    pub(crate) const MAXP: Self = Self(*b"maxp");
    pub(crate) const OS2: Self = Self(*b"OS/2");
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            let shown = if byte.is_ascii_graphic() || byte == b' ' {
                char::from(byte)
            } else {
                '?'
            };
            write!(f, "{shown}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tag({self})")
    }
}

/// The `N` bytes at `offset`.
fn bytes<const N: usize>(data: &[u8], offset: usize) -> Option<[u8; N]> {
    let end = offset.checked_add(N)?;
    data.get(offset..end)?.try_into().ok()
}

pub(crate) fn u8_at(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

pub(crate) fn i8_at(data: &[u8], offset: usize) -> Option<i8> {
    bytes::<1>(data, offset).map(i8::from_be_bytes)
}

pub(crate) fn u16_at(data: &[u8], offset: usize) -> Option<u16> {
    bytes(data, offset).map(u16::from_be_bytes)
}

pub(crate) fn i16_at(data: &[u8], offset: usize) -> Option<i16> {
    bytes(data, offset).map(i16::from_be_bytes)
}

pub(crate) fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    bytes(data, offset).map(u32::from_be_bytes)
}

/// The `len` bytes at `offset`.
pub(crate) fn slice(data: &[u8], offset: usize, len: usize) -> Option<&[u8]> {
    data.get(offset..offset.checked_add(len)?)
}

/// A cursor over big-endian data.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn at(data: &'a [u8], offset: usize) -> Self {
        Self { data, offset }
    }

    fn advance(&mut self, len: usize) -> Option<usize> {
        let start = self.offset;
        self.offset = start.checked_add(len)?;
        Some(start)
    }

    pub(crate) fn u8(&mut self) -> Option<u8> {
        let at = self.advance(1)?;
        u8_at(self.data, at)
    }

    pub(crate) fn i8(&mut self) -> Option<i8> {
        let at = self.advance(1)?;
        i8_at(self.data, at)
    }

    pub(crate) fn u16(&mut self) -> Option<u16> {
        let at = self.advance(2)?;
        u16_at(self.data, at)
    }

    pub(crate) fn i16(&mut self) -> Option<i16> {
        let at = self.advance(2)?;
        i16_at(self.data, at)
    }

    pub(crate) fn skip(&mut self, len: usize) -> Option<()> {
        self.advance(len)?;
        (self.offset <= self.data.len()).then_some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_past_the_end_return_none() {
        let data = [0x12, 0x34, 0xFF];
        assert_eq!(u16_at(&data, 0), Some(0x1234));
        assert_eq!(i16_at(&data, 1), Some(0x34FF));
        assert_eq!(u16_at(&data, 2), None);
        assert_eq!(u32_at(&data, 0), None);
        assert_eq!(u16_at(&data, usize::MAX), None);
        assert_eq!(slice(&data, 1, 2), Some(&data[1..3]));
        assert_eq!(slice(&data, 2, usize::MAX), None);
        let mut reader = Reader::at(&data, 0);
        assert_eq!(reader.i8(), Some(0x12));
        assert_eq!(reader.u16(), Some(0x34FF));
        assert_eq!(reader.u8(), None);
        assert_eq!(Reader::at(&data, 0).skip(4), None);
    }

    #[test]
    fn tags_print_as_text() {
        assert_eq!(Tag::OS2.to_string(), "OS/2");
        assert_eq!(Tag([b'a', 0, b'b', 0xFF]).to_string(), "a?b?");
    }
}

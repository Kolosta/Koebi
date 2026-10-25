//! Binary encoding primitives of the persistence format: little-endian numbers,
//! length-prefixed UTF-8 strings and the CRC-32 checksum.

use crate::error::{Error, Result};

/// Appends values to a byte buffer.
#[derive(Debug, Default)]
pub(crate) struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn f32(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// A count or length, as `u32` (saturating; collections here are far smaller).
    pub(crate) fn len(&mut self, len: usize) {
        self.u32(u32::try_from(len).unwrap_or(u32::MAX));
    }

    pub(crate) fn str(&mut self, text: &str) {
        self.len(text.len());
        self.bytes.extend_from_slice(text.as_bytes());
    }
}

/// Reads values from a byte slice; every read fails cleanly at the end of the data.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
}

/// Longest string accepted (a token path or a theme description).
const MAX_STRING: usize = 1 << 16;
/// Largest count accepted (themes, overrides, masks).
const MAX_COUNT: usize = 1 << 20;

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.bytes.len() < len {
            return Err(Error::Corrupt("unexpected end of data".to_owned()));
        }
        let (head, tail) = self.bytes.split_at(len);
        self.bytes = tail;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut array = [0; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        self.array().map(u16::from_le_bytes)
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        self.array().map(u32::from_le_bytes)
    }

    pub(crate) fn u64(&mut self) -> Result<u64> {
        self.array().map(u64::from_le_bytes)
    }

    pub(crate) fn i32(&mut self) -> Result<i32> {
        self.array().map(i32::from_le_bytes)
    }

    pub(crate) fn f32(&mut self) -> Result<f32> {
        self.array().map(f32::from_le_bytes)
    }

    /// A count written by [`Writer::len`], bounded to reject absurd values early.
    pub(crate) fn count(&mut self) -> Result<usize> {
        let count = usize::try_from(self.u32()?).unwrap_or(usize::MAX);
        if count > MAX_COUNT {
            return Err(Error::Corrupt(format!("implausible count {count}")));
        }
        Ok(count)
    }

    pub(crate) fn str(&mut self) -> Result<&'a str> {
        let len = usize::try_from(self.u32()?).unwrap_or(usize::MAX);
        if len > MAX_STRING {
            return Err(Error::Corrupt(format!("implausible string length {len}")));
        }
        core::str::from_utf8(self.take(len)?)
            .map_err(|_| Error::Corrupt("text is not valid UTF-8".to_owned()))
    }
}

/// CRC-32 (IEEE 802.3, the checksum of PNG and zip files).
pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0_u32; 256];
        let mut n: u32 = 0;
        while n < 256 {
            let mut c = n;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 == 1 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[n as usize] = c;
            n += 1;
        }
        table
    };
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc = TABLE[usize::from(crc.to_le_bytes()[0] ^ byte)] ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip() {
        let mut writer = Writer::default();
        writer.u8(7);
        writer.u16(0xBEEF);
        writer.u32(123_456);
        writer.u64(u64::MAX);
        writer.i32(-42);
        writer.f32(0.25);
        writer.str("background.base.default");
        let bytes = writer.into_bytes();
        let mut reader = Reader::new(&bytes);
        assert_eq!(reader.u8().unwrap(), 7);
        assert_eq!(reader.u16().unwrap(), 0xBEEF);
        assert_eq!(reader.u32().unwrap(), 123_456);
        assert_eq!(reader.u64().unwrap(), u64::MAX);
        assert_eq!(reader.i32().unwrap(), -42);
        assert_eq!(reader.f32().unwrap().to_bits(), 0.25_f32.to_bits());
        assert_eq!(reader.str().unwrap(), "background.base.default");
        assert!(reader.is_empty());
        assert!(matches!(reader.u8(), Err(Error::Corrupt(_))));
    }

    #[test]
    fn implausible_sizes_are_rejected() {
        let mut writer = Writer::default();
        writer.u32(u32::MAX);
        let bytes = writer.into_bytes();
        assert!(Reader::new(&bytes).str().is_err());
        assert!(Reader::new(&bytes).count().is_err());
        assert!(
            Reader::new(&[0xFF, 0, 0, 0, 0xC3]).str().is_err(),
            "truncated"
        );
    }

    #[test]
    fn crc32_matches_the_reference_value() {
        // The standard check value of CRC-32/ISO-HDLC.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }
}

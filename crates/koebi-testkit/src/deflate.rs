//! Raw deflate (RFC 1951) as the PNG codec needs it: compression with LZ77 matches and the fixed
//! Huffman codes, and decompression of stored and fixed-code blocks. Dynamic-code blocks, which
//! other tools write, are refused: files are only ever written by this codec.

use crate::png::PngError;

/// The window matches are looked for in, in bytes.
const WINDOW: usize = 32_768;
/// The longest match deflate encodes.
const MAX_MATCH: usize = 258;
/// The shortest match worth encoding.
const MIN_MATCH: usize = 3;
/// How many earlier positions of the same three bytes are tried, at most.
const MAX_CHAIN: usize = 64;

/// Base length of each length code (257 to 285) and its extra bits.
const LENGTHS: [(u16, u8); 29] = [
    (3, 0),
    (4, 0),
    (5, 0),
    (6, 0),
    (7, 0),
    (8, 0),
    (9, 0),
    (10, 0),
    (11, 1),
    (13, 1),
    (15, 1),
    (17, 1),
    (19, 2),
    (23, 2),
    (27, 2),
    (31, 2),
    (35, 3),
    (43, 3),
    (51, 3),
    (59, 3),
    (67, 4),
    (83, 4),
    (99, 4),
    (115, 4),
    (131, 5),
    (163, 5),
    (195, 5),
    (227, 5),
    (258, 0),
];

/// Base distance of each distance code (0 to 29) and its extra bits.
const DISTANCES: [(u16, u8); 30] = [
    (1, 0),
    (2, 0),
    (3, 0),
    (4, 0),
    (5, 1),
    (7, 1),
    (9, 2),
    (13, 2),
    (17, 3),
    (25, 3),
    (33, 4),
    (49, 4),
    (65, 5),
    (97, 5),
    (129, 6),
    (193, 6),
    (257, 7),
    (385, 7),
    (513, 8),
    (769, 8),
    (1025, 9),
    (1537, 9),
    (2049, 10),
    (3073, 10),
    (4097, 11),
    (6145, 11),
    (8193, 12),
    (12289, 12),
    (16385, 13),
    (24577, 13),
];

/// Bits written least significant first, as deflate packs them.
struct BitWriter {
    out: Vec<u8>,
    bits: u32,
    count: u32,
}

impl BitWriter {
    fn put(&mut self, value: u32, count: u32) {
        self.bits |= value << self.count;
        self.count += count;
        while self.count >= 8 {
            self.out.push(self.bits.to_le_bytes()[0]);
            self.bits >>= 8;
            self.count -= 8;
        }
    }

    /// A Huffman code, written most significant bit first.
    fn code(&mut self, code: u32, length: u32) {
        self.put(code.reverse_bits() >> (32 - length), length);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.count > 0 {
            self.out.push(self.bits.to_le_bytes()[0]);
        }
        self.out
    }
}

/// The fixed code of a literal or length symbol (0 to 287): `(code, length)`.
fn literal_code(symbol: u16) -> (u32, u32) {
    let symbol = u32::from(symbol);
    match symbol {
        0..=143 => (0x30 + symbol, 8),
        144..=255 => (0x190 + symbol - 144, 9),
        256..=279 => (symbol - 256, 7),
        _ => (0xC0 + symbol - 280, 8),
    }
}

/// The code and extra bits of a match length or distance in `table`: `(index, extra, bits)`.
fn bucket(value: u16, table: &[(u16, u8)]) -> (u16, u16, u8) {
    let index = table
        .iter()
        .rposition(|&(base, _)| base <= value)
        .unwrap_or(0);
    let (base, bits) = table[index];
    (u16::try_from(index).unwrap_or(u16::MAX), value - base, bits)
}

/// `data` compressed in one fixed-code block.
pub(crate) fn compress(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter {
        out: Vec::with_capacity(data.len() / 4 + 16),
        bits: 0,
        count: 0,
    };
    // BFINAL = 1, BTYPE = 01 (fixed codes).
    writer.put(1, 1);
    writer.put(1, 2);
    // The last position of each three-byte prefix, and for each position the one before it.
    let mut head = vec![usize::MAX; 1 << 15];
    let mut previous = vec![usize::MAX; data.len()];
    let hash = |at: usize| {
        let key =
            (u32::from(data[at]) << 16) | (u32::from(data[at + 1]) << 8) | u32::from(data[at + 2]);
        (key.wrapping_mul(0x9E37_79B1) >> 17) as usize
    };
    let mut at = 0;
    while at < data.len() {
        let mut best = (0, 0);
        if at + MIN_MATCH <= data.len() {
            let slot = hash(at);
            let mut candidate = head[slot];
            let mut tries = 0;
            while candidate != usize::MAX && at - candidate <= WINDOW && tries < MAX_CHAIN {
                let limit = MAX_MATCH.min(data.len() - at);
                let length = (0..limit)
                    .take_while(|&offset| data[candidate + offset] == data[at + offset])
                    .count();
                if length > best.0 {
                    best = (length, at - candidate);
                    if length == limit {
                        break;
                    }
                }
                candidate = previous[candidate];
                tries += 1;
            }
            previous[at] = head[slot];
            head[slot] = at;
        }
        let (length, distance) = best;
        if length >= MIN_MATCH {
            let length16 = u16::try_from(length).unwrap_or(258);
            let (code, extra, bits) = bucket(length16, &LENGTHS);
            let (literal, size) = literal_code(257 + code);
            writer.code(literal, size);
            writer.put(u32::from(extra), u32::from(bits));
            let (code, extra, bits) = bucket(u16::try_from(distance).unwrap_or(1), &DISTANCES);
            writer.code(u32::from(code), 5);
            writer.put(u32::from(extra), u32::from(bits));
            // The positions inside the match join the chains too.
            let last = (at + length).min((data.len() + 1).saturating_sub(MIN_MATCH));
            for (inside, link) in previous.iter_mut().enumerate().take(last).skip(at + 1) {
                let slot = hash(inside);
                *link = head[slot];
                head[slot] = inside;
            }
            at += length;
        } else {
            let (literal, size) = literal_code(u16::from(data[at]));
            writer.code(literal, size);
            at += 1;
        }
    }
    let (end, size) = literal_code(256);
    writer.code(end, size);
    writer.finish()
}

/// Bits read least significant first.
struct BitReader<'a> {
    data: &'a [u8],
    at: usize,
    bit: u32,
}

impl BitReader<'_> {
    fn bit(&mut self) -> Result<u32, PngError> {
        let byte = *self.data.get(self.at).ok_or(PngError::Truncated)?;
        let value = u32::from(byte >> self.bit) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.at += 1;
        }
        Ok(value)
    }

    fn bits(&mut self, count: u8) -> Result<u32, PngError> {
        let mut value = 0;
        for index in 0..count {
            value |= self.bit()? << index;
        }
        Ok(value)
    }

    /// `count` bits of a Huffman code, most significant first.
    fn code(&mut self, count: u8) -> Result<u32, PngError> {
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | self.bit()?;
        }
        Ok(value)
    }

    /// A literal or length symbol of the fixed code.
    fn literal(&mut self) -> Result<u16, PngError> {
        let mut code = self.code(7)?;
        if code <= 0x17 {
            return Ok(u16::try_from(256 + code).unwrap_or(0));
        }
        code = (code << 1) | self.bit()?;
        let symbol = match code {
            0x30..=0xBF => code - 0x30,
            0xC0..=0xC7 => 280 + code - 0xC0,
            _ => {
                code = (code << 1) | self.bit()?;
                if !(0x190..=0x1FF).contains(&code) {
                    return Err(PngError::Corrupt("deflate code"));
                }
                144 + code - 0x190
            }
        };
        Ok(u16::try_from(symbol).unwrap_or(0))
    }
}

/// `data`, raw deflate of stored and fixed-code blocks, decompressed; and where it ended.
pub(crate) fn inflate(data: &[u8]) -> Result<(Vec<u8>, usize), PngError> {
    let mut reader = BitReader {
        data,
        at: 0,
        bit: 0,
    };
    let mut out = Vec::new();
    loop {
        let last = reader.bit()? == 1;
        match reader.bits(2)? {
            0 => {
                // Stored: aligned on a byte, its length twice.
                if reader.bit != 0 {
                    reader.bit = 0;
                    reader.at += 1;
                }
                let header = data
                    .get(reader.at..reader.at + 4)
                    .ok_or(PngError::Truncated)?;
                let len = u16::from_le_bytes([header[0], header[1]]);
                if len != !u16::from_le_bytes([header[2], header[3]]) {
                    return Err(PngError::Corrupt("stored block length"));
                }
                let start = reader.at + 4;
                let block = data
                    .get(start..start + usize::from(len))
                    .ok_or(PngError::Truncated)?;
                out.extend_from_slice(block);
                reader.at = start + usize::from(len);
            }
            1 => loop {
                let symbol = reader.literal()?;
                match symbol {
                    0..=255 => out.push(u8::try_from(symbol).unwrap_or(0)),
                    256 => break,
                    _ => {
                        let &(base, bits) = LENGTHS
                            .get(usize::from(symbol - 257))
                            .ok_or(PngError::Corrupt("deflate length"))?;
                        let length = usize::from(base) + reader.bits(bits)? as usize;
                        let code = reader.code(5)? as usize;
                        let &(base, bits) = DISTANCES
                            .get(code)
                            .ok_or(PngError::Corrupt("deflate distance"))?;
                        let distance = usize::from(base) + reader.bits(bits)? as usize;
                        if distance > out.len() {
                            return Err(PngError::Corrupt("deflate distance"));
                        }
                        let from = out.len() - distance;
                        for offset in 0..length {
                            out.push(out[from + offset]);
                        }
                    }
                }
            },
            _ => return Err(PngError::Unsupported("dynamic-code deflate blocks")),
        }
        if last {
            break;
        }
    }
    let end = reader.at + usize::from(reader.bit > 0);
    Ok((out, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_comes_back_from_compression() {
        let mut varied = Vec::new();
        for index in 0_u32..20_000 {
            // Runs, repeats far apart and noise.
            varied.push(u8::try_from((index / 7) % 251).unwrap());
            if index % 97 == 0 {
                varied.extend_from_slice(b"repeated text, repeated text");
            }
            varied.push(u8::try_from(index.wrapping_mul(2_654_435_761) >> 24).unwrap());
        }
        for data in [
            Vec::new(),
            vec![0],
            vec![7; 100_000],
            b"abcabcabcabcabcabc".to_vec(),
            varied,
        ] {
            let compressed = compress(&data);
            let (back, end) = inflate(&compressed).unwrap();
            assert_eq!(back, data);
            assert_eq!(end, compressed.len());
        }
        // Long runs compress to almost nothing.
        assert!(compress(&vec![0; 100_000]).len() < 1_000);
    }

    #[test]
    fn stored_blocks_and_their_errors_are_read() {
        // A stored block: BFINAL, BTYPE 00, the length and its complement, the bytes.
        let stored = [1, 3, 0, 0xFC, 0xFF, b'a', b'b', b'c'];
        assert_eq!(inflate(&stored).unwrap(), (b"abc".to_vec(), 8));
        assert!(inflate(&[1, 3, 0, 0, 0, b'a']).is_err());
        // A dynamic-code block.
        assert!(matches!(inflate(&[0b101]), Err(PngError::Unsupported(_))));
    }
}

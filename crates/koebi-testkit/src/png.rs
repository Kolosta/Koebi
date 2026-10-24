//! Minimal PNG codec for golden images (RGBA8, uncompressed deflate "stored" blocks).
//!
//! Golden images are only ever written by [`encode_png`] (via `cargo xtask bless`), so the
//! decoder supports exactly what the encoder produces plus all five row filters. Files
//! re-compressed by external tools are rejected with a message telling how to regenerate them.

use std::fmt;

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const MAX_STORED_BLOCK: usize = 65_535;

/// PNG decoding failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PngError {
    /// The data does not start with the PNG signature.
    NotPng,
    /// The data ends in the middle of a structure.
    Truncated,
    /// Valid PNG using a feature this codec does not implement.
    Unsupported(&'static str),
    /// Checksums or structure are invalid.
    Corrupt(&'static str),
}

impl fmt::Display for PngError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPng => f.write_str("not a PNG file"),
            Self::Truncated => f.write_str("truncated PNG file"),
            Self::Unsupported(what) => write!(
                f,
                "unsupported PNG feature: {what} (regenerate golden images with `cargo xtask bless`)"
            ),
            Self::Corrupt(what) => write!(f, "corrupt PNG file: {what}"),
        }
    }
}

impl std::error::Error for PngError {}

/// Encodes RGBA8 pixels as a PNG with stored (uncompressed) deflate blocks.
///
/// # Panics
///
/// Panics if `rgba.len() != width * height * 4` or the image is absurdly large.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let stride = width as usize * 4;
    assert_eq!(
        rgba.len(),
        stride * height as usize,
        "RGBA buffer size mismatch"
    );

    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in rgba.chunks_exact(stride.max(1)) {
        raw.push(0); // filter type: none
        raw.extend_from_slice(row);
    }

    let mut zlib = vec![0x78, 0x01];
    if raw.is_empty() {
        zlib.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    let mut blocks = raw.chunks(MAX_STORED_BLOCK).peekable();
    while let Some(block) = blocks.next() {
        zlib.push(u8::from(blocks.peek().is_none())); // BFINAL, BTYPE = 00 (stored)
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit, RGBA, deflate, adaptive, no interlace

    let mut png = SIGNATURE.to_vec();
    write_chunk(&mut png, *b"IHDR", &header);
    write_chunk(&mut png, *b"IDAT", &zlib);
    write_chunk(&mut png, *b"IEND", &[]);
    png
}

/// Decodes a PNG produced by [`encode_png`]. Returns `(width, height, rgba)`.
///
/// # Errors
///
/// Returns a [`PngError`] for non-PNG data, checksum mismatches, compressed data or pixel
/// formats other than 8-bit RGBA.
pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), PngError> {
    let mut rest = bytes.strip_prefix(&SIGNATURE).ok_or(PngError::NotPng)?;
    let mut size = None;
    let mut idat = Vec::new();
    loop {
        let (len, after) = split_u32(rest)?;
        let len = len as usize;
        if after.len() < len + 8 {
            return Err(PngError::Truncated);
        }
        let (kind, after) = after.split_at(4);
        let (data, after) = after.split_at(len);
        let (crc, after) = split_u32(after)?;
        rest = after;
        if crc32(&[kind, data]) != crc {
            return Err(PngError::Corrupt("chunk CRC mismatch"));
        }
        match kind {
            b"IHDR" => size = Some(parse_header(data)?),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            // Critical chunks have an upper-case first letter and must be understood.
            other if other[0].is_ascii_uppercase() => {
                return Err(PngError::Unsupported("critical chunk"));
            }
            _ => {}
        }
    }
    let (width, height) = size.ok_or(PngError::Corrupt("missing IHDR"))?;
    let raw = inflate_stored(&idat)?;
    let rgba = unfilter(&raw, width as usize, height as usize)?;
    Ok((width, height, rgba))
}

fn parse_header(data: &[u8]) -> Result<(u32, u32), PngError> {
    if data.len() != 13 {
        return Err(PngError::Corrupt("IHDR length"));
    }
    let (width, rest) = split_u32(data)?;
    let (height, rest) = split_u32(rest)?;
    match rest {
        [8, 6, 0, 0, 0] => Ok((width, height)),
        [_, _, _, _, 1] => Err(PngError::Unsupported("interlacing")),
        _ => Err(PngError::Unsupported("pixel format other than 8-bit RGBA")),
    }
}

fn inflate_stored(zlib: &[u8]) -> Result<Vec<u8>, PngError> {
    if zlib.len() < 2 {
        return Err(PngError::Truncated);
    }
    let (cmf, flg) = (zlib[0], zlib[1]);
    if cmf & 0x0F != 8 || (u16::from(cmf) << 8 | u16::from(flg)) % 31 != 0 {
        return Err(PngError::Corrupt("zlib header"));
    }
    if flg & 0x20 != 0 {
        return Err(PngError::Unsupported("zlib preset dictionary"));
    }
    let mut out = Vec::new();
    let mut pos = 2;
    loop {
        let block = zlib.get(pos..pos + 5).ok_or(PngError::Truncated)?;
        let header = block[0];
        if (header >> 1) & 0b11 != 0 {
            return Err(PngError::Unsupported("compressed deflate blocks"));
        }
        let len = u16::from_le_bytes([block[1], block[2]]);
        if len != !u16::from_le_bytes([block[3], block[4]]) {
            return Err(PngError::Corrupt("stored block length"));
        }
        let start = pos + 5;
        let data = zlib
            .get(start..start + usize::from(len))
            .ok_or(PngError::Truncated)?;
        out.extend_from_slice(data);
        pos = start + usize::from(len);
        if header & 1 == 1 {
            break;
        }
    }
    let (checksum, _) = split_u32(&zlib[pos..])?;
    if checksum != adler32(&out) {
        return Err(PngError::Corrupt("adler32 mismatch"));
    }
    Ok(out)
}

fn unfilter(raw: &[u8], width: usize, height: usize) -> Result<Vec<u8>, PngError> {
    const BPP: usize = 4;
    let stride = width * BPP;
    if raw.len() != (stride + 1) * height {
        return Err(PngError::Corrupt("image data size"));
    }
    let mut out = vec![0_u8; stride * height];
    for y in 0..height {
        let filter = raw[y * (stride + 1)];
        let line = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        let (done, current) = out.split_at_mut(y * stride);
        let previous = (y > 0).then(|| &done[(y - 1) * stride..]);
        let current = &mut current[..stride];
        for x in 0..stride {
            let a = if x >= BPP { current[x - BPP] } else { 0 };
            let b = previous.map_or(0, |p| p[x]);
            let c = if x >= BPP {
                previous.map_or(0, |p| p[x - BPP])
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => u8::try_from(u16::midpoint(u16::from(a), u16::from(b))).unwrap_or(0),
                4 => paeth(a, b, c),
                _ => return Err(PngError::Corrupt("unknown row filter")),
            };
            current[x] = line[x].wrapping_add(predictor);
        }
    }
    Ok(out)
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = (
        (p - i16::from(a)).abs(),
        (p - i16::from(b)).abs(),
        (p - i16::from(c)).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn split_u32(bytes: &[u8]) -> Result<(u32, &[u8]), PngError> {
    match bytes {
        [a, b, c, d, rest @ ..] => Ok((u32::from_be_bytes([*a, *b, *c, *d]), rest)),
        _ => Err(PngError::Truncated),
    }
}

fn write_chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&[&kind, data]).to_be_bytes());
}

const CRC_TABLE: [u32; 256] = {
    let mut table = [0_u32; 256];
    let mut n = 0;
    while n < 256 {
        #[allow(clippy::cast_possible_truncation, reason = "n < 256")]
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for part in parts {
        for &byte in *part {
            crc = CRC_TABLE[((crc ^ u32::from(byte)) & 0xFF) as usize] ^ (crc >> 8);
        }
    }
    crc ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_match_reference_values() {
        assert_eq!(crc32(&[b"IEND"]), 0xAE42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn encode_decode_round_trip() {
        let (width, height) = (5_u32, 3_u32);
        let rgba: Vec<u8> = (0..width * height * 4)
            .map(|i| u8::try_from(i * 7 % 256).unwrap())
            .collect();
        let png = encode_png(width, height, &rgba);
        assert_eq!(decode_png(&png).unwrap(), (width, height, rgba));
    }

    #[test]
    fn large_images_span_several_stored_blocks() {
        let (width, height) = (200_u32, 100_u32); // 80 kB of pixels > one 64 kB block
        let rgba = vec![0xAB; (width * height * 4) as usize];
        let (w, h, decoded) = decode_png(&encode_png(width, height, &rgba)).unwrap();
        assert_eq!((w, h), (width, height));
        assert_eq!(decoded, rgba);
    }

    #[test]
    fn corrupted_or_foreign_data_is_rejected() {
        assert_eq!(decode_png(b"GIF89a"), Err(PngError::NotPng));
        let mut png = encode_png(1, 1, &[1, 2, 3, 4]);
        let last = png.len() - 20;
        png[last] ^= 0xFF;
        assert!(decode_png(&png).is_err());
    }

    #[test]
    fn row_filters_are_reversed() {
        // One 2x2 image encoded with the Sub filter on row 0 and the Up filter on row 1.
        let raw = [1, 10, 20, 30, 40, 5, 5, 5, 5, 2, 1, 1, 1, 1, 2, 2, 2, 2];
        let out = unfilter(&raw, 2, 2).unwrap();
        assert_eq!(&out[..8], &[10, 20, 30, 40, 15, 25, 35, 45]);
        assert_eq!(&out[8..], &[11, 21, 31, 41, 17, 27, 37, 47]);
    }
}

//! Minimal, dependency-free PNG encoder (8-bit RGBA, stored deflate blocks).
//!
//! Only used as a fallback so `polyforge-paint` can save textures without
//! pulling in an external image crate. Compression is intentionally absent
//! (zlib "stored" blocks), which keeps the writer tiny and predictable.

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    assert_eq!(rgba.len(), (width * height * 4) as usize);

    let mut out = Vec::with_capacity(rgba.len() + 1024);
    out.extend_from_slice(&SIGNATURE);

    // IHDR: width, height, bit depth 8, colour type 6 (RGBA), no interlace.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // colour type RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut out, b"IHDR", &ihdr);

    // Raw scanlines: one filter byte (0) then RGBA pixels.
    let stride = (width * 4) as usize;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for y in 0..height as usize {
        raw.push(0);
        let start = y * stride;
        raw.extend_from_slice(&rgba[start..start + stride]);
    }

    write_chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// zlib stream using stored (uncompressed) deflate blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 65535 * 5 + 16);
    out.push(0x78); // CMF: deflate, 32K window
    out.push(0x01); // FLG: no dict, fastest

    if data.is_empty() {
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
    } else {
        let mut offset = 0;
        while offset < data.len() {
            let remaining = data.len() - offset;
            let len = remaining.min(0xFFFF);
            let final_block = offset + len >= data.len();
            out.push(if final_block { 0x01 } else { 0x00 });
            let len_u16 = len as u16;
            out.extend_from_slice(&len_u16.to_le_bytes());
            out.extend_from_slice(&(!len_u16).to_le_bytes());
            out.extend_from_slice(&data[offset..offset + len]);
            offset += len;
        }
    }

    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adler_is_well_known_value() {
        // Adler-32 of the ASCII string "Wikipedia".
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn png_header_and_chunks() {
        let px = vec![255u8; 4 * 4]; // 2x2 white
        let png = encode_png(2, 2, &px);
        assert_eq!(&png[..8], &SIGNATURE);
        assert_eq!(&png[12..16], b"IHDR");
        assert!(png.windows(4).any(|w| w == b"IDAT"));
        assert!(png.windows(4).any(|w| w == b"IEND"));
    }
}

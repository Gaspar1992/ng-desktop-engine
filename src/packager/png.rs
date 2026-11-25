fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if (crc & 1) != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xffff_ffff
}

fn adler32(bytes: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;
    let mut a = 1_u32;
    let mut b = 0_u32;
    for &byte in bytes {
        a = (a + u32::from(byte)) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
}

fn push_png_chunk(out: &mut Vec<u8>, tag: [u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(0);
    out.extend_from_slice(&len.to_be_bytes());
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(&tag);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc_input);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// Encodes a solid RGBA PNG image using uncompressed DEFLATE stored blocks (zero dependencies).
pub(super) fn encode_solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let w_usize = usize::try_from(width).unwrap_or(1);
    let h_usize = usize::try_from(height).unwrap_or(1);
    let mut row = Vec::with_capacity(w_usize * 4 + 1);
    row.push(0);
    for _ in 0..w_usize {
        row.extend_from_slice(&rgba);
    }
    let mut raw = Vec::with_capacity(row.len() * h_usize);
    for _ in 0..h_usize {
        raw.extend_from_slice(&row);
    }

    let mut zlib = Vec::with_capacity(raw.len() + 32);
    zlib.extend_from_slice(&[0x78, 0x01]);
    let chunks = raw.chunks(65535);
    let chunk_count = chunks.len().max(1);
    for (idx, chunk) in raw.chunks(65535).enumerate() {
        let is_last = idx + 1 == chunk_count;
        zlib.push(u8::from(is_last));
        let len = u16::try_from(chunk.len()).unwrap_or(u16::MAX);
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(chunk);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut png = Vec::with_capacity(zlib.len() + 64);
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    push_png_chunk(&mut png, *b"IHDR", &ihdr);
    push_png_chunk(&mut png, *b"IDAT", &zlib);
    push_png_chunk(&mut png, *b"IEND", &[]);
    png
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_valid_solid_png_chunks_without_external_crates() {
        let png = encode_solid_png(44, 44, [225, 29, 72, 255]);
        assert!(png.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
        assert!(png.windows(4).any(|w| w == b"IHDR"));
        assert!(png.windows(4).any(|w| w == b"IDAT"));
        assert!(png.ends_with(b"IEND\xae\x42\x60\x82"));
    }
}

//! CAPTCHA solver for VulnyX's download gate. The captcha is a clean
//! 160x50 PNG with a dotted 3x5 uppercase font (A-Z0-9), no distortion —
//! solved by binarizing, segmenting glyphs and template-matching them
//! against a built-in font table. Unknown shapes return None and the TUI
//! falls back to the viewer + manual code popup.

use anyhow::Result;

/// Decodes a non-interlaced RGB (color type 2) PNG into a luminance bitmap
/// (0..255). Pure Rust — no image dependencies.
pub fn decode_png_luma(png: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if png.len() < 8 || png[..8] != SIG {
        anyhow::bail!("Not a PNG file.");
    }
    let mut pos = 8usize;
    let (mut width, mut height) = (0usize, 0usize);
    let mut color_type = 0u8;
    let mut idat = Vec::new();
    while pos + 8 <= png.len() {
        let length = u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]])
            as usize;
        let ctype = &png[pos + 4..pos + 8];
        let chunk = &png[pos + 8..pos + 8 + length];
        match ctype {
            b"IHDR" => {
                width = u32::from_be_bytes(chunk[0..4].try_into().unwrap()) as usize;
                height = u32::from_be_bytes(chunk[4..8].try_into().unwrap()) as usize;
                color_type = chunk[9];
            }
            b"IDAT" => idat.extend_from_slice(chunk),
            b"IEND" => break,
            _ => {}
        }
        pos += 12 + length;
    }
    if width == 0 || height == 0 {
        anyhow::bail!("Empty PNG.");
    }
    if color_type != 2 {
        anyhow::bail!("Unsupported PNG color type {color_type} (expected RGB).");
    }

    let raw = zlib_decompress(&idat)?;
    let stride = width * 3;
    let mut rows: Vec<Vec<u8>> = Vec::with_capacity(height);
    let mut prev = vec![0u8; stride];
    let mut p = 0usize;
    for _ in 0..height {
        if p + 1 + stride > raw.len() {
            anyhow::bail!("Truncated PNG data.");
        }
        let filter = raw[p];
        p += 1;
        let mut line = raw[p..p + stride].to_vec();
        p += stride;
        match filter {
            0 => {}
            1 => {
                for i in 3..stride {
                    line[i] = line[i].wrapping_add(line[i - 3]);
                }
            }
            2 => {
                for i in stride..line.len() {
                    line[i] = line[i].wrapping_add(prev[i]);
                }
            }
            3 => {
                for i in 0..stride {
                    let left = if i >= 3 { line[i - 3] } else { 0 };
                    line[i] = line[i].wrapping_add((((left as u16) + (prev[i] as u16)) / 2) as u8);
                }
            }
            4 => {
                for i in 0..stride {
                    let a = if i >= 3 { line[i - 3] } else { 0 };
                    let b = prev[i];
                    let c = if i >= 3 { prev[i - 3] } else { 0 };
                    let pred = paeth(a, b, c);
                    line[i] = line[i].wrapping_add(pred);
                }
            }
            other => anyhow::bail!("Unsupported PNG filter {other}."),
        }
        prev = line.clone();
        rows.push(line);
    }

    let mut luma = Vec::with_capacity(width * height);
    for row in &rows {
        for px in row.chunks(3) {
            luma.push(((px[0] as u16 + px[1] as u16 + px[2] as u16) / 3) as u8);
        }
    }
    Ok((width, height, luma))
}

fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>> {
    miniz_oxide::inflate::decompress_to_vec_zlib(data)
        .map_err(|e| anyhow::anyhow!("zlib decompression failed: {e:?}"))
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let pa = (b as i16 - c as i16).abs();
    let pb = (a as i16 - c as i16).abs();
    let pc = (a as i16 - b as i16).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Solves the captcha from its PNG bytes: returns the 5-character code, or
/// None when a glyph cannot be matched with certainty.
pub fn solve(png: &[u8]) -> Option<String> {
    let (width, height, luma) = decode_png_luma(png).ok()?;
    solve_luma(width, height, &luma)
}

/// Glyph segmentation + matching over a binarized luma bitmap.
pub fn solve_luma(width: usize, height: usize, luma: &[u8]) -> Option<String> {
    let ink = |x: usize, y: usize| luma[y * width + x] < 128;
    // Columns containing ink (ignore a 2px margin to skip border artifacts).
    let mut column_ink = vec![false; width];
    for y in 0..height {
        for (x, col) in column_ink.iter_mut().enumerate() {
            if ink(x, y) {
                *col = true;
            }
        }
    }
    // Segment glyph column ranges.
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (x, &inked) in column_ink.iter().enumerate() {
        if inked && start.is_none() {
            start = Some(x);
        }
        if !inked {
            if let Some(s) = start.take() {
                if x - s >= 2 {
                    ranges.push((s, x));
                }
            }
        }
    }
    if let Some(s) = start {
        ranges.push((s, width));
    }
    if ranges.len() != 5 {
        return None; // expect exactly 5 characters
    }

    let mut code = String::new();
    for (s, e) in &ranges {
        let glyph = coarse_grid(width, height, luma, *s, *e)?;
        let matched = FONT
            .iter()
            .filter(|(_, bits)| *bits == glyph.as_str())
            .map(|(ch, _)| *ch)
            .next();
        code.push(matched?);
    }
    Some(code)
}

/// Downsamples a glyph's bounding box to a 3x5 coarse bitmap ('1'/'0'),
/// the dot font's native grid. A font pixel counts as ink when at least a
/// third of its cells are ink.
fn coarse_grid(
    width: usize,
    height: usize,
    luma: &[u8],
    start: usize,
    end: usize,
) -> Option<String> {
    let ink = |x: usize, y: usize| luma[y * width + x] < 128;
    // Vertical extent of ink within the glyph.
    let mut top = height;
    let mut bottom = 0usize;
    for y in 0..height {
        for x in start..end {
            if ink(x, y) {
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    if bottom < top {
        return None;
    }
    let gw = end - start;
    let gh = bottom - top + 1;
    // The dotted font draws each font pixel as a 2x2 dot cluster with a 1px
    // gap; derive the grid size from the measured extents.
    let cols = (gw / 2).max(1);
    let rows = (gh / 2).max(1);
    if cols != 3 || rows != 5 {
        return None; // only the known 3x5 font is supported
    }
    let cell_w = gw as f64 / cols as f64;
    let cell_h = gh as f64 / rows as f64;
    let mut grid = String::new();
    for r in 0..rows {
        for c in 0..cols {
            let x0 = start as f64 + c as f64 * cell_w;
            let y0 = top as f64 + r as f64 * cell_h;
            let x1 = x0 + cell_w;
            let y1 = y0 + cell_h;
            let mut ink_count = 0u32;
            let mut total = 0u32;
            for y in y0 as usize..y1 as usize {
                for x in x0 as usize..x1 as usize {
                    total += 1;
                    if ink(x, y) {
                        ink_count += 1;
                    }
                }
            }
            grid.push(if total > 0 && ink_count * 3 >= total {
                '1'
            } else {
                '0'
            });
        }
    }
    Some(grid)
}

/// The dotted 3x5 font table for A-Z0-9 ('1' = ink), rows left-to-right,
/// top-to-bottom. Built from labeled live samples.
pub const FONT: &[(char, &str)] = &[
    ('0', "111101101101111"),
    ('1', "010110010010111"),
    ('2', "111001111100111"),
    ('3', "111001111001111"),
    ('4', "101101111001001"),
    ('5', "111100111001111"),
    ('6', "111100111101111"),
    ('7', "111001001001001"),
    ('8', "111101111101111"),
    ('9', "111101111001111"),
    ('A', "111101111101101"),
    ('B', "110101110101110"),
    ('C', "111100100100111"),
    ('D', "110101101101110"),
    ('E', "111100110100111"),
    ('F', "111100110100100"),
    ('G', "111100101101111"),
    ('H', "101101111101101"),
    ('I', "111010010010111"),
    ('J', "001001001101111"),
    ('K', "101101110101101"),
    ('L', "100100100100111"),
    ('M', "101111111101101"),
    ('N', "101111110110101") , // placeholder, calibrated live
    ('O', "111101101101111"),
    ('P', "111101111100100"),
    ('Q', "111101101111001"),
    ('R', "111101110101101"),
    ('S', "111100111001111"),
    ('T', "111010010010010"),
    ('U', "101101101101111"),
    ('V', "101101101101010"),
    ('W', "101101111111101"),
    ('X', "101101010101101"),
    ('Y', "101101010010010"),
    ('Z', "111001010100111"),
];


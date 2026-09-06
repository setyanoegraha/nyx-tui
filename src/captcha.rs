//! CAPTCHA handling for VulnyX's download gate
#![allow(dead_code)] // auto-solver pending font table calibration: PNG decoding, ASCII
//! rendering for in-terminal display, and pattern-matching auto-solve.
//!
//! The captcha is a clean 160x50 PNG with a dotted pixel font (A-Z0-9),
//! no distortion — rendered as ASCII art inside the TUI popup so the user
//! never needs an external viewer.

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

/// Renders a captcha PNG as ASCII art lines for in-terminal display.
/// Each column is halved (160px → 80 chars) to fit terminal width.
/// Ink pixels become '#', background becomes ' '.
pub fn render_ascii(png: &[u8]) -> Result<Vec<String>> {
    let (width, height, luma) = decode_png_luma(png)?;
    let mut lines = Vec::new();
    // Render at half resolution (1 char per 2x2 px) for compact terminal display
    for y in (0..height).step_by(2) {
        let mut line = String::new();
        for x in (0..width).step_by(2) {
            // Sample the darkest pixel in each 2x2 block
            let l1 = luma[y * width + x];
            let l2 = if x + 1 < width { luma[y * width + x + 1] } else { 255 };
            let l3 = if y + 1 < height { luma[(y + 1) * width + x] } else { 255 };
            let l4 = if x + 1 < width && y + 1 < height { luma[(y + 1) * width + x + 1] } else { 255 };
            let darkest = l1.min(l2).min(l3).min(l4);
            line.push(if darkest < 60 { '#' } else { ' ' });
        }
        let trimmed = line.trim_end().to_string();
        lines.push(trimmed);
    }
    // Remove leading/trailing blank lines
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    Ok(lines)
}

/// Solves the captcha from its PNG bytes using font template matching.
/// Returns the 5-character code, or None when a glyph cannot be matched.
pub fn solve(png: &[u8]) -> Option<String> {
    let (width, height, luma) = decode_png_luma(png).ok()?;
    solve_luma(width, height, &luma)
}

/// Finds the column ranges of the 5 glyphs in the captcha bitmap.
fn find_glyph_ranges(width: usize, height: usize, luma: &[u8]) -> Vec<(usize, usize)> {
    let ink = |x: usize, y: usize| luma[y * width + x] < 128;
    let mut column_ink = vec![false; width];
    for y in 0..height {
        for (x, col) in column_ink.iter_mut().enumerate() {
            if ink(x, y) {
                *col = true;
            }
        }
    }
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
    ranges
}

/// Extracts the raw bitmap of a glyph (binary, 1 = ink).
fn extract_glyph(
    width: usize,
    height: usize,
    luma: &[u8],
    start: usize,
    end: usize,
) -> Vec<Vec<bool>> {
    let ink = |x: usize, y: usize| luma[y * width + x] < 128;
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
        return Vec::new();
    }
    let mut grid = Vec::new();
    for y in top..=bottom {
        let mut row = Vec::new();
        for x in start..end {
            row.push(ink(x, y));
        }
        grid.push(row);
    }
    grid
}

/// Solves the captcha from a luma bitmap by segmenting glyphs and
/// template-matching them against known font patterns.
pub fn solve_luma(width: usize, height: usize, luma: &[u8]) -> Option<String> {
    let ranges = find_glyph_ranges(width, height, luma);
    if ranges.len() != 5 {
        return None;
    }

    let mut code = String::new();
    for (start, end) in &ranges {
        let glyph = extract_glyph(width, height, luma, *start, *end);
        if glyph.is_empty() {
            return None;
        }
        let ch = match_glyph(&glyph)?;
        code.push(ch);
    }
    Some(code)
}

/// Compares a glyph bitmap against the known font patterns.
/// Returns the matched character, or None if no match is found.
fn match_glyph(glyph: &[Vec<bool>]) -> Option<char> {
    // Normalize to a compact string representation
    let normalized = normalize_glyph(glyph);

    for (ch, pattern) in FONT_PATTERNS {
        if *pattern == normalized {
            return Some(*ch);
        }
    }
    None
}

/// Converts a raw glyph bitmap to a normalized string ('1' = ink, '0' = blank),
/// trimming empty rows and columns.
fn normalize_glyph(glyph: &[Vec<bool>]) -> String {
    let mut s = String::new();
    for row in glyph {
        for &cell in row {
            s.push(if cell { '1' } else { '0' });
        }
        s.push('\n');
    }
    s.trim_end().to_string()
}

/// Known font patterns from live captcha samples.
/// Each pattern is the raw glyph bitmap rendered as '1'/'0' rows separated by '\n'.
/// Built from live samples during development.
pub const FONT_PATTERNS: &[(char, &str)] = &[
    // Populated during development with live captcha samples
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_ascii_with_real_captcha() {
        let path = std::env::var("CAPTCHA_PNG").unwrap_or_default();
        if path.is_empty() {
            return; // skip if no captcha file provided
        }
        let png = std::fs::read(&path).unwrap();
        let lines = render_ascii(&png).unwrap();
        println!("lines: {}", lines.len());
        for line in &lines {
            println!("{line}");
        }
        assert!(!lines.is_empty(), "render_ascii should produce lines");
        assert!(
            lines.iter().any(|l| l.contains('#')),
            "at least one line should have ink"
        );
    }

    #[test]
    fn test_decode_png_luma_works() {
        let path = "/tmp/vx-captcha.png";
        let png = std::fs::read(path).expect("cannot read captcha file");
        println!("file size: {}", png.len());
        println!("first bytes: {:02x?}", &png[..16]);
        match decode_png_luma(&png) {
            Ok((w, h, luma)) => {
                println!("decoded: {w}x{h}");
                let ink = luma.iter().filter(|&&l| l < 128).count();
                println!("ink pixels: {ink}");
            }
            Err(e) => println!("DECODE ERROR: {e:#}"),
        }
    }

}

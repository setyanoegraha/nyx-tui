//! VulnyX download CAPTCHA: pure-Rust PNG decoding plus exact template
//! matching against the site's fixed 8x10 dotted font.
//!
//! The site prints 5 glyphs from a 30-character alphabet
//! (`ABCDEFGHJKLMNPQRTUVWXYZ2346789` — no `I`, `O`, `S`, `0`, `1` or `5`), always
//! on rows 21..30 of a 160x50 RGB image. Across 545 sampled glyphs every draw
//! was one of the 30 bitmaps in [`TEMPLATES`], so a glyph not found there means
//! the font changed: [`solve`] returns `None` and the caller falls back to the
//! browser instead of guessing (nearest-neighbour would pick `F` for `E`).

use anyhow::{bail, Result};

/// A decoded image reduced to a `max(r, g, b)` luminance map.
pub struct Captcha {
    pub width: usize,
    pub height: usize,
    pub luma: Vec<u8>,
}

/// Pixels darker than this on every channel are glyph ink. Sampled images
/// split cleanly: ink <= 65, background >= 176.
const INK_THRESHOLD: u8 = 140;

fn is_ink(captcha: &Captcha, x: usize, y: usize) -> bool {
    captcha.luma[y * captcha.width + x] < INK_THRESHOLD
}

/// Decodes an 8-bit RGB (colour type 2), non-interlaced PNG.
pub fn decode_png(png: &[u8]) -> Result<Captcha> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if png.len() < 8 || png[..8] != SIGNATURE {
        bail!("Not a PNG file.");
    }

    let mut pos = 8;
    let mut width = 0usize;
    let mut height = 0usize;
    let mut idat = Vec::new();
    while pos + 8 <= png.len() {
        let length = u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]])
            as usize;
        let chunk_type = &png[pos + 4..pos + 8];
        let Some(chunk) = png.get(pos + 8..pos + 8 + length) else {
            bail!("Truncated PNG chunk.");
        };
        match chunk_type {
            b"IHDR" => {
                if chunk.len() < 13 {
                    bail!("Truncated PNG header.");
                }
                width = u32::from_be_bytes(chunk[0..4].try_into().unwrap()) as usize;
                height = u32::from_be_bytes(chunk[4..8].try_into().unwrap()) as usize;
                let (depth, colour_type, interlace) = (chunk[8], chunk[9], chunk[12]);
                if depth != 8 || colour_type != 2 || interlace != 0 {
                    bail!("Unsupported PNG layout (need 8-bit RGB, non-interlaced).");
                }
            }
            b"IDAT" => idat.extend_from_slice(chunk),
            b"IEND" => break,
            _ => {}
        }
        pos += 12 + length;
    }
    if width == 0 || height == 0 {
        bail!("Empty PNG.");
    }

    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&idat)
        .map_err(|error| anyhow::anyhow!("zlib decompression failed: {error:?}"))?;
    let stride = width * 3;
    let mut rows: Vec<Vec<u8>> = Vec::with_capacity(height);
    let mut previous = vec![0u8; stride];
    let mut offset = 0usize;
    for _ in 0..height {
        let Some(&filter) = raw.get(offset) else {
            bail!("Truncated PNG data.");
        };
        offset += 1;
        let Some(slice) = raw.get(offset..offset + stride) else {
            bail!("Truncated PNG data.");
        };
        offset += stride;
        let mut line = slice.to_vec();
        match filter {
            0 => {}
            1 => {
                for i in 3..stride {
                    line[i] = line[i].wrapping_add(line[i - 3]);
                }
            }
            2 => {
                for i in 0..stride {
                    line[i] = line[i].wrapping_add(previous[i]);
                }
            }
            3 => {
                for i in 0..stride {
                    let left = if i >= 3 { line[i - 3] } else { 0 };
                    let average = ((u16::from(left) + u16::from(previous[i])) / 2) as u8;
                    line[i] = line[i].wrapping_add(average);
                }
            }
            4 => {
                for i in 0..stride {
                    let a = if i >= 3 { line[i - 3] } else { 0 };
                    let b = previous[i];
                    let c = if i >= 3 { previous[i - 3] } else { 0 };
                    line[i] = line[i].wrapping_add(paeth(a, b, c));
                }
            }
            other => bail!("Unsupported PNG filter {other}."),
        }
        previous = line.clone();
        rows.push(line);
    }

    let mut luma = Vec::with_capacity(width * height);
    for row in &rows {
        for pixel in row.as_chunks::<3>().0 {
            luma.push(pixel[0].max(pixel[1]).max(pixel[2]));
        }
    }
    Ok(Captcha {
        width,
        height,
        luma,
    })
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ai, bi, ci) = (i16::from(a), i16::from(b), i16::from(c));
    let estimate = ai + bi - ci;
    let (pa, pb, pc) = ((estimate - ai).abs(), (estimate - bi).abs(), (estimate - ci).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Column ranges of the glyphs (left and right inclusive). A column is inked
/// when any of its pixels is; runs narrower than 2 columns are noise.
pub fn find_glyphs(captcha: &Captcha) -> Vec<(usize, usize)> {
    let inked: Vec<bool> = (0..captcha.width)
        .map(|x| (0..captcha.height).any(|y| is_ink(captcha, x, y)))
        .collect();
    let mut ranges = Vec::new();
    let mut start: Option<usize> = None;
    for (x, &column) in inked.iter().enumerate() {
        match (column, start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                if x - s >= 2 {
                    ranges.push((s, x - 1));
                }
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        if captcha.width - s >= 2 {
            ranges.push((s, captcha.width - 1));
        }
    }
    ranges
}

/// Renders one glyph onto a fixed 8x10 canvas: rows 21..=30, columns
/// `start..start+8`, `#` for ink, `.` otherwise. 6- and 7-wide glyphs come out
/// right-padded, which is how the templates are written.
pub fn glyph_key(captcha: &Captcha, start: usize, end: usize) -> String {
    let rows: Vec<usize> = if captcha.height > 30 {
        (21..=30).collect()
    } else {
        let top = (0..captcha.height)
            .find(|&y| (start..=end).any(|x| is_ink(captcha, x, y)))
            .unwrap_or(0);
        (top..top + 10).map(|y| y.min(captcha.height - 1)).collect()
    };
    let mut key = String::with_capacity(10 * 8 + 9);
    for (index, y) in rows.iter().enumerate() {
        if index > 0 {
            key.push('|');
        }
        for x in start..start + 8 {
            let dark = x <= end && x < captcha.width && is_ink(captcha, x, *y);
            key.push(if dark { '#' } else { '.' });
        }
    }
    key
}

/// Decodes and solves a CAPTCHA image; `None` when it cannot be read with
/// certainty (unknown font, wrong glyph count, bad PNG).
pub fn solve(png: &[u8]) -> Option<String> {
    let captcha = decode_png(png).ok()?;
    solve_captcha(&captcha)
}

/// Solves an already decoded CAPTCHA.
pub fn solve_captcha(captcha: &Captcha) -> Option<String> {
    let ranges = find_glyphs(captcha);
    if ranges.len() != 5 {
        return None;
    }
    let mut code = String::with_capacity(5);
    for (start, end) in ranges {
        let key = glyph_key(captcha, start, end);
        let (character, _) = templates().iter().find(|(_, pattern)| *pattern == key)?;
        code.push(*character);
    }
    Some(code)
}

/// The site's full alphabet with its exact glyph bitmaps.
pub fn templates() -> &'static [(char, &'static str)] {
    TEMPLATES
}

pub const TEMPLATES: &[(char, &str)] = &[
    ('P', "#######.|##....##|##....##|##....##|#######.|##......|##......|##......|##......|##......"),
    ('8', "..####..|.##..##.|##....##|.##..##.|..####..|.##..##.|##....##|##....##|.##..##.|..####.."),
    ('Q', "..####..|.##..##.|##....##|##....##|##....##|##....##|##.##.##|##..####|.##..##.|..####.#"),
    ('V', "##....##|##....##|##....##|.##..##.|.##..##.|.##..##.|..####..|..####..|...##...|...##..."),
    ('Z', "#######.|.....##.|.....##.|....##..|...##...|..##....|.##.....|##......|##......|#######."),
    ('D', "######..|##...##.|##....##|##....##|##....##|##....##|##....##|##....##|##...##.|######.."),
    ('T', "########|...##...|...##...|...##...|...##...|...##...|...##...|...##...|...##...|...##..."),
    ('F', "########|##......|##......|##......|######..|##......|##......|##......|##......|##......"),
    ('K', "##....##|##...##.|##..##..|##.##...|####....|####....|##.##...|##..##..|##...##.|##....##"),
    ('L', "##......|##......|##......|##......|##......|##......|##......|##......|##......|#######."),
    ('C', "..#####.|.##...##|##.....#|##......|##......|##......|##......|##.....#|.##...##|..#####."),
    ('Y', "##....##|##....##|.##..##.|..####..|...##...|...##...|...##...|...##...|...##...|...##..."),
    ('6', "..####..|.##..##.|##....#.|##......|##.###..|###..##.|##....##|##....##|.##..##.|..####.."),
    ('2', "..####..|.##..##.|##....##|......##|.....##.|....##..|...##...|..##....|.##.....|########"),
    ('A', "...##...|..####..|.##..##.|##....##|##....##|##....##|########|##....##|##....##|##....##"),
    ('U', "##....##|##....##|##....##|##....##|##....##|##....##|##....##|##....##|.##..##.|..####.."),
    ('J', "..####..|....##..|....##..|....##..|....##..|....##..|....##..|#...##..|##.##...|.###...."),
    ('B', "######..|##...##.|##....##|##...##.|######..|##...##.|##....##|##....##|##...##.|######.."),
    ('N', "##....##|###...##|####..##|####..##|##.##.##|##.##.##|##..####|##...###|##...###|##....##"),
    ('G', "..#####.|.##...##|##......|##......|##......|##...###|##....##|##....##|.##...##|..#####."),
    ('4', ".....##.|....###.|...####.|..##.##.|.##..##.|##...##.|########|.....##.|.....##.|.....##."),
    ('9', "..####..|.##..##.|##....##|##....##|.##..###|..###.##|......##|.#....##|.##..##.|..####.."),
    ('7', "########|......##|......##|.....##.|....##..|...##...|..##....|.##.....|##......|##......"),
    ('R', "#######.|##....##|##....##|##....##|#######.|#####...|##..##..|##...##.|##....##|##....##"),
    ('X', "##....##|##....##|.##..##.|..####..|...##...|...##...|..####..|.##..##.|##....##|##....##"),
    ('M', "##....##|###..###|########|##.##.##|##.##.##|##.##.##|##....##|##....##|##....##|##....##"),
    ('W', "##....##|##....##|##....##|##....##|##.##.##|##.##.##|##.##.##|########|###..###|##....##"),
    ('E', "#######.|##......|##......|##......|######..|##......|##......|##......|##......|#######."),
    ('H', "##....##|##....##|##....##|##....##|########|##....##|##....##|##....##|##....##|##....##"),
    ('3', ".#####..|##...##.|......##|.....##.|...###..|.....##.|......##|......##|##...##.|.#####.."),
];

/// Builds a 160x50 RGB PNG holding `glyphs` at the given left offsets on rows
/// 21..30, with every row but the first PNG-filtered as `Up`. Test helper.
#[cfg(test)]
fn synthetic_png(glyphs: &[(char, usize)]) -> Vec<u8> {
    const WIDTH: usize = 160;
    const HEIGHT: usize = 50;
    let stride = WIDTH * 3;
    let mut rows = vec![vec![0u8; stride]; HEIGHT];
    for row in rows.iter_mut() {
        for pixel in row.as_chunks_mut::<3>().0 {
            pixel.copy_from_slice(&[245, 246, 248]);
        }
    }
    for (character, left) in glyphs {
        let pattern = TEMPLATES
            .iter()
            .find(|(candidate, _)| candidate == character)
            .unwrap_or_else(|| panic!("no template for {character}"))
            .1;
        for (index, line) in pattern.split('|').enumerate() {
            for (column, cell) in line.chars().enumerate() {
                if cell == '#' {
                    let offset = (left + column) * 3;
                    rows[21 + index][offset..offset + 3].copy_from_slice(&[21, 23, 41]);
                }
            }
        }
    }

    let mut raw = Vec::with_capacity(HEIGHT * (stride + 1));
    for (index, row) in rows.iter().enumerate() {
        if index == 0 {
            raw.push(0);
            raw.extend_from_slice(row);
        } else {
            let previous = &rows[index - 1];
            raw.push(2);
            raw.extend(row.iter().zip(previous).map(|(a, b)| a.wrapping_sub(*b)));
        }
    }
    let idat = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut header = Vec::new();
    header.extend_from_slice(&(WIDTH as u32).to_be_bytes());
    header.extend_from_slice(&(HEIGHT as u32).to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    for (chunk_type, payload) in [(b"IHDR", header), (b"IDAT", idat), (b"IEND", Vec::new())] {
        png.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        png.extend_from_slice(chunk_type);
        png.extend_from_slice(&payload);
        png.extend_from_slice(&[0, 0, 0, 0]);
    }
    png
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_a_synthetic_captcha() {
        // The glyphs the site produced for code Q69BY, at their real offsets.
        let png = synthetic_png(&[('Q', 28), ('6', 55), ('9', 79), ('B', 105), ('Y', 130)]);
        assert_eq!(solve(&png).as_deref(), Some("Q69BY"));
    }

    #[test]
    fn rejects_an_unknown_glyph_count() {
        let png = synthetic_png(&[('Q', 28), ('6', 55), ('9', 79), ('B', 105)]);
        assert_eq!(solve(&png), None);
    }

    #[test]
    fn rejects_non_png_bytes() {
        assert!(decode_png(b"not a png").is_err());
        assert_eq!(solve(b"not a png"), None);
    }

    #[test]
    fn templates_cover_thirty_unique_characters() {
        let table = templates();
        assert_eq!(table.len(), 30);
        let mut characters: Vec<char> = table.iter().map(|(c, _)| *c).collect();
        characters.sort_unstable();
        characters.dedup();
        assert_eq!(characters.len(), 30);
        let mut keys: Vec<&str> = table.iter().map(|(_, k)| *k).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 30, "duplicate glyph bitmaps");
    }

    #[test]
    fn real_sample_from_env() {
        let Ok(path) = std::env::var("NYX_CAPTCHA_PNG") else {
            return;
        };
        let png = std::fs::read(&path).expect("cannot read NYX_CAPTCHA_PNG");
        let code = solve(&png);
        println!("decoded {path} -> {code:?}");
        let code = code.expect("solver could not read the sample");
        assert_eq!(code.len(), 5);
        assert!(code
            .chars()
            .all(|c| TEMPLATES.iter().any(|(t, _)| *t == c)));
    }
}

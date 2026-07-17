//! Anti-aliased text rendering for the standalone SDL2 frontend.
//!
//! The module deliberately uses system fonts instead of SDL_ttf, keeping the
//! release free of an additional native DLL. If no supported font is present,
//! a small built-in bitmap font keeps the UI functional.

use fontdue::{Font, FontSettings};
use std::path::PathBuf;
use std::sync::OnceLock;

const MAX_FONT_SIZE_PX: usize = 512;

static SYSTEM_FONT: OnceLock<Option<Font>> = OnceLock::new();

/// Draw `text` with its top-left corner at `(x, y)`.
///
/// The framebuffer and `color_argb` use straight-alpha `0xAARRGGBB`. Drawing
/// is clipped against both the declared canvas and the actual slice length.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text(
    framebuffer: &mut [u32],
    canvas_w: usize,
    canvas_h: usize,
    x: usize,
    y: usize,
    size_px: usize,
    color_argb: u32,
    text: &str,
) {
    if framebuffer.is_empty()
        || canvas_w == 0
        || canvas_h == 0
        || x >= canvas_w
        || y >= canvas_h
        || text.is_empty()
        || color_argb >> 24 == 0
    {
        return;
    }

    if let Some(font) = system_font() {
        draw_fontdue_text(
            font,
            framebuffer,
            canvas_w,
            canvas_h,
            x,
            y,
            size_px,
            color_argb,
            text,
        );
    } else {
        draw_bitmap_text(
            framebuffer,
            canvas_w,
            canvas_h,
            x,
            y,
            size_px,
            color_argb,
            text,
        );
    }
}

/// Return the width of the widest line in `text`, in pixels.
pub(crate) fn text_width(text: &str, size_px: usize) -> usize {
    if text.is_empty() {
        return 0;
    }

    system_font().map_or_else(
        || bitmap_text_width(text, size_px),
        |font| fontdue_text_width(font, text, size_px),
    )
}

/// Wrap text to `max_width`, preserving explicit line breaks.
///
/// Whitespace inside a paragraph is normalized to a single space. Words wider
/// than the requested width are split at Unicode scalar boundaries, ensuring
/// the function always makes progress.
pub(crate) fn wrap_text(text: &str, max_width: usize, size_px: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if max_width == 0 {
        return normalized
            .split('\n')
            .map(|line| line.trim().to_owned())
            .collect();
    }

    let mut lines = Vec::new();
    for paragraph in normalized.split('\n') {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            if current.is_empty() {
                place_word(&mut lines, &mut current, word, max_width, size_px);
                continue;
            }

            let mut candidate = String::with_capacity(current.len() + word.len() + 1);
            candidate.push_str(&current);
            candidate.push(' ');
            candidate.push_str(word);
            if text_width(&candidate, size_px) <= max_width {
                current = candidate;
            } else {
                lines.push(std::mem::take(&mut current));
                place_word(&mut lines, &mut current, word, max_width, size_px);
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}

fn system_font() -> Option<&'static Font> {
    SYSTEM_FONT.get_or_init(load_system_font).as_ref()
}

fn load_system_font() -> Option<Font> {
    system_font_candidates().into_iter().find_map(|path| {
        let bytes = std::fs::read(path).ok()?;
        Font::from_bytes(bytes, FontSettings::default()).ok()
    })
}

fn system_font_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(custom) = std::env::var_os("NGNEON_FONT") {
        paths.push(PathBuf::from(custom));
    }

    #[cfg(target_os = "windows")]
    {
        let windows = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        let fonts = windows.join("Fonts");
        for name in [
            "bahnschrift.ttf",
            "segoeui.ttf",
            "segoeuib.ttf",
            "arial.ttf",
        ] {
            paths.push(fonts.join(name));
        }

        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let user_fonts = PathBuf::from(local_app_data)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts");
            for name in ["bahnschrift.ttf", "segoeui.ttf", "arial.ttf"] {
                paths.push(user_fonts.join(name));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        for path in [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
        ] {
            paths.push(PathBuf::from(path));
        }
    }

    #[cfg(target_os = "macos")]
    {
        for path in [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/Supplemental/Helvetica.ttf",
            "/System/Library/Fonts/Supplemental/Verdana.ttf",
            "/Library/Fonts/Arial.ttf",
        ] {
            paths.push(PathBuf::from(path));
        }
    }

    paths
}

#[allow(clippy::too_many_arguments)]
fn draw_fontdue_text(
    font: &Font,
    framebuffer: &mut [u32],
    canvas_w: usize,
    canvas_h: usize,
    x: usize,
    y: usize,
    size_px: usize,
    color_argb: u32,
    text: &str,
) {
    let size = sanitized_size(size_px);
    let (ascent, line_height) = font
        .horizontal_line_metrics(size)
        .map(|metrics| (metrics.ascent, metrics.new_line_size.max(size)))
        .unwrap_or((size * 0.8, size * 1.2));
    let space_advance = font.metrics(' ', size).advance_width.max(1.0);
    let mut cursor_x = x as f32;
    let mut baseline = y as f32 + ascent;

    for character in text.chars() {
        match character {
            '\r' => continue,
            '\n' => {
                cursor_x = x as f32;
                baseline += line_height;
                continue;
            }
            '\t' => {
                cursor_x += space_advance * 4.0;
                continue;
            }
            _ => {}
        }

        let (metrics, bitmap) = font.rasterize(character, size);
        let glyph_x = cursor_x.round() as i64 + i64::from(metrics.xmin);
        let glyph_y = baseline.round() as i64 - metrics.height as i64 - i64::from(metrics.ymin);

        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let coverage = bitmap[row * metrics.width + column];
                if coverage == 0 {
                    continue;
                }
                blend_coverage_pixel(
                    framebuffer,
                    canvas_w,
                    canvas_h,
                    glyph_x + column as i64,
                    glyph_y + row as i64,
                    color_argb,
                    coverage,
                );
            }
        }
        cursor_x += metrics.advance_width.max(0.0);
    }
}

fn fontdue_text_width(font: &Font, text: &str, size_px: usize) -> usize {
    let size = sanitized_size(size_px);
    let space_advance = font.metrics(' ', size).advance_width.max(1.0);
    let mut widest = 0.0_f32;
    let mut current = 0.0_f32;

    for character in text.chars() {
        match character {
            '\r' => {}
            '\n' => {
                widest = widest.max(current);
                current = 0.0;
            }
            '\t' => current += space_advance * 4.0,
            _ => current += font.metrics(character, size).advance_width.max(0.0),
        }
    }
    widest.max(current).ceil() as usize
}

fn sanitized_size(size_px: usize) -> f32 {
    size_px.clamp(1, MAX_FONT_SIZE_PX) as f32
}

fn place_word(
    lines: &mut Vec<String>,
    current: &mut String,
    word: &str,
    max_width: usize,
    size_px: usize,
) {
    if text_width(word, size_px) <= max_width {
        current.push_str(word);
        return;
    }

    let mut chunks = split_long_word(word, max_width, size_px);
    if let Some(last) = chunks.pop() {
        lines.extend(chunks);
        *current = last;
    }
}

fn split_long_word(word: &str, max_width: usize, size_px: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for character in word.chars() {
        let mut candidate = current.clone();
        candidate.push(character);
        if !current.is_empty() && text_width(&candidate, size_px) > max_width {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(character);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn blend_coverage_pixel(
    framebuffer: &mut [u32],
    canvas_w: usize,
    canvas_h: usize,
    x: i64,
    y: i64,
    color_argb: u32,
    coverage: u8,
) {
    let Ok(x) = usize::try_from(x) else {
        return;
    };
    let Ok(y) = usize::try_from(y) else {
        return;
    };
    if x >= canvas_w || y >= canvas_h {
        return;
    }
    let Some(index) = y.checked_mul(canvas_w).and_then(|row| row.checked_add(x)) else {
        return;
    };
    let Some(destination) = framebuffer.get_mut(index) else {
        return;
    };

    let color_alpha = color_argb >> 24;
    let effective_alpha = (color_alpha * u32::from(coverage) + 127) / 255;
    if effective_alpha == 0 {
        return;
    }
    let source = (effective_alpha << 24) | (color_argb & 0x00FF_FFFF);
    *destination = source_over(source, *destination);
}

/// Straight-alpha source-over composition for `0xAARRGGBB` pixels.
fn source_over(source: u32, destination: u32) -> u32 {
    let source_alpha = source >> 24;
    if source_alpha == 0 {
        return destination;
    }
    if source_alpha == 255 {
        return source;
    }

    let destination_alpha = destination >> 24;
    let inverse_source_alpha = 255 - source_alpha;
    let output_alpha_numerator = source_alpha * 255 + destination_alpha * inverse_source_alpha;
    if output_alpha_numerator == 0 {
        return 0;
    }

    let source_red = (source >> 16) & 0xFF;
    let source_green = (source >> 8) & 0xFF;
    let source_blue = source & 0xFF;
    let destination_red = (destination >> 16) & 0xFF;
    let destination_green = (destination >> 8) & 0xFF;
    let destination_blue = destination & 0xFF;

    let blend_channel = |source_channel: u32, destination_channel: u32| {
        let numerator = source_channel * source_alpha * 255
            + destination_channel * destination_alpha * inverse_source_alpha;
        (numerator + output_alpha_numerator / 2) / output_alpha_numerator
    };
    let output_alpha = (output_alpha_numerator + 127) / 255;
    let output_red = blend_channel(source_red, destination_red);
    let output_green = blend_channel(source_green, destination_green);
    let output_blue = blend_channel(source_blue, destination_blue);

    (output_alpha << 24) | (output_red << 16) | (output_green << 8) | output_blue
}

#[allow(clippy::too_many_arguments)]
fn draw_bitmap_text(
    framebuffer: &mut [u32],
    canvas_w: usize,
    canvas_h: usize,
    x: usize,
    y: usize,
    size_px: usize,
    color_argb: u32,
    text: &str,
) {
    let scale = bitmap_scale(size_px);
    let advance = 6 * scale;
    let line_height = 8 * scale;
    let mut cursor_x = x;
    let mut cursor_y = y;

    for character in text.chars() {
        match character {
            '\r' => continue,
            '\n' => {
                cursor_x = x;
                cursor_y = cursor_y.saturating_add(line_height);
                continue;
            }
            '\t' => {
                cursor_x = cursor_x.saturating_add(advance * 4);
                continue;
            }
            _ => {}
        }

        let glyph = bitmap_glyph(character);
        for (column, bits) in glyph.iter().enumerate() {
            for row in 0..7 {
                if bits & (1 << row) == 0 {
                    continue;
                }
                for offset_y in 0..scale {
                    for offset_x in 0..scale {
                        blend_coverage_pixel(
                            framebuffer,
                            canvas_w,
                            canvas_h,
                            cursor_x
                                .saturating_add(column * scale)
                                .saturating_add(offset_x) as i64,
                            cursor_y
                                .saturating_add(row * scale)
                                .saturating_add(offset_y) as i64,
                            color_argb,
                            255,
                        );
                    }
                }
            }
        }
        cursor_x = cursor_x.saturating_add(advance);
    }
}

fn bitmap_text_width(text: &str, size_px: usize) -> usize {
    let advance = 6 * bitmap_scale(size_px);
    let mut widest = 0usize;
    let mut current = 0usize;
    for character in text.chars() {
        match character {
            '\r' => {}
            '\n' => {
                widest = widest.max(current);
                current = 0;
            }
            '\t' => current = current.saturating_add(advance * 4),
            _ => current = current.saturating_add(advance),
        }
    }
    widest.max(current)
}

fn bitmap_scale(size_px: usize) -> usize {
    ((size_px.clamp(1, MAX_FONT_SIZE_PX) as f32 / 7.0).round() as usize).max(1)
}

type BitmapGlyph = [u8; 5];

fn bitmap_glyph(character: char) -> &'static BitmapGlyph {
    let index = match character {
        '0'..='9' => character as usize - '0' as usize,
        'A'..='Z' => 10 + (character as usize - 'A' as usize),
        'a'..='z' => 10 + (character as usize - 'a' as usize),
        ' ' | '_' => 36,
        ':' => 37,
        '.' => 38,
        '/' | '\\' => 39,
        '-' => 40,
        '(' => 41,
        ')' => 42,
        '\u{2190}' => 43,
        '\u{2192}' => 44,
        '\u{2191}' => 45,
        '\u{2193}' => 46,
        '>' => 47,
        '%' => 48,
        '\u{2026}' => 49,
        '!' => 50,
        '?' => 51,
        '\u{00f1}' | '\u{00d1}' => 52,
        '\u{00e1}' | '\u{00c1}' => 53,
        '\u{00e9}' | '\u{00c9}' => 54,
        '\u{00ed}' | '\u{00cd}' => 55,
        '\u{00f3}' | '\u{00d3}' => 56,
        '\u{00fa}' | '\u{00da}' => 57,
        _ => 51,
    };
    &BITMAP_FONT[index]
}

#[rustfmt::skip]
const BITMAP_FONT: [BitmapGlyph; 58] = [
    [0x3E,0x45,0x49,0x51,0x3E], [0x00,0x42,0x7F,0x40,0x00], [0x62,0x51,0x49,0x49,0x46], [0x22,0x41,0x49,0x49,0x36],
    [0x18,0x14,0x12,0x7F,0x10], [0x27,0x45,0x45,0x45,0x39], [0x3E,0x49,0x49,0x49,0x32], [0x01,0x71,0x09,0x05,0x03],
    [0x36,0x49,0x49,0x49,0x36], [0x26,0x49,0x49,0x49,0x3E],
    [0x7C,0x12,0x11,0x12,0x7C], [0x7F,0x49,0x49,0x49,0x36], [0x3E,0x41,0x41,0x41,0x22], [0x7F,0x41,0x41,0x41,0x3E],
    [0x7F,0x49,0x49,0x49,0x41], [0x7F,0x09,0x09,0x09,0x01], [0x3E,0x41,0x49,0x49,0x3A], [0x7F,0x08,0x08,0x08,0x7F],
    [0x00,0x41,0x7F,0x41,0x00], [0x20,0x40,0x41,0x41,0x3F], [0x7F,0x08,0x14,0x22,0x41], [0x7F,0x40,0x40,0x40,0x40],
    [0x7F,0x02,0x04,0x02,0x7F], [0x7F,0x02,0x04,0x08,0x7F], [0x3E,0x41,0x41,0x41,0x3E], [0x7F,0x09,0x09,0x09,0x06],
    [0x3E,0x41,0x51,0x21,0x5E], [0x7F,0x09,0x19,0x29,0x46], [0x26,0x49,0x49,0x49,0x32], [0x01,0x01,0x7F,0x01,0x01],
    [0x3F,0x40,0x40,0x40,0x3F], [0x07,0x08,0x30,0x08,0x07], [0x1F,0x20,0x18,0x20,0x1F], [0x41,0x22,0x1C,0x22,0x41],
    [0x03,0x04,0x78,0x04,0x03], [0x61,0x51,0x49,0x45,0x43],
    [0x00,0x00,0x00,0x00,0x00], [0x00,0x36,0x36,0x00,0x00], [0x00,0x60,0x60,0x00,0x00], [0x40,0x30,0x0C,0x03,0x00],
    [0x08,0x08,0x08,0x08,0x08], [0x1C,0x22,0x41,0x00,0x00], [0x41,0x22,0x1C,0x00,0x00], [0x08,0x1C,0x2A,0x08,0x08],
    [0x08,0x08,0x2A,0x1C,0x08], [0x08,0x1C,0x3E,0x08,0x08], [0x08,0x08,0x3E,0x1C,0x08], [0x00,0x08,0x1C,0x08,0x00],
    [0x42,0x25,0x12,0x48,0x21], [0x00,0x00,0x00,0x00,0x6D], [0x00,0x00,0x5F,0x00,0x00], [0x02,0x01,0x51,0x09,0x06],
    [0x7F,0x02,0x04,0x08,0x7F], [0x7C,0x12,0x11,0x12,0x7C], [0x7F,0x49,0x49,0x49,0x41], [0x00,0x41,0x7F,0x41,0x00],
    [0x3E,0x41,0x41,0x41,0x3E], [0x3F,0x40,0x40,0x40,0x3F],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_over_blends_half_white_over_opaque_black() {
        assert_eq!(source_over(0x80FF_FFFF, 0xFF00_0000), 0xFF80_8080);
    }

    #[test]
    fn source_over_handles_transparent_edges() {
        let destination = 0x7F12_3456;
        assert_eq!(source_over(0x0011_2233, destination), destination);
        assert_eq!(source_over(0xFFAB_CDEF, destination), 0xFFAB_CDEF);
    }

    #[test]
    fn text_width_uses_the_widest_explicit_line() {
        let expected = text_width("METAL SLUG", 18).max(text_width("III", 18));
        assert_eq!(text_width("METAL SLUG\nIII", 18), expected);
    }

    #[test]
    fn wrapping_respects_width_and_preserves_words() {
        let max_width = text_width("METAL SLUG", 16);
        let lines = wrap_text("METAL SLUG THREE ARCADE", max_width, 16);
        assert!(lines.len() >= 2);
        assert!(lines.iter().all(|line| text_width(line, 16) <= max_width));
        assert_eq!(lines.join(" "), "METAL SLUG THREE ARCADE");
    }

    #[test]
    fn wrapping_splits_a_word_that_is_too_wide() {
        let max_width = text_width("META", 16).max(1);
        let lines = wrap_text("METALSLUG", max_width, 16);
        assert!(lines.len() >= 2);
        assert!(lines.iter().all(|line| text_width(line, 16) <= max_width));
        assert_eq!(lines.concat(), "METALSLUG");
    }

    #[test]
    fn explicit_blank_lines_are_preserved() {
        assert_eq!(wrap_text("UNO\n\nDOS", 1_000, 16), ["UNO", "", "DOS"]);
    }

    #[test]
    fn visible_text_changes_pixels_inside_the_canvas() {
        let mut framebuffer = vec![0xFF00_0000; 64 * 32];
        draw_text(&mut framebuffer, 64, 32, 2, 2, 20, 0xFFFF_FFFF, "A");
        assert!(framebuffer.iter().any(|pixel| *pixel != 0xFF00_0000));
    }

    #[test]
    fn transparent_text_and_clipped_coordinates_are_safe() {
        let mut framebuffer = vec![0xFF12_3456; 16];
        draw_text(&mut framebuffer, 4, 4, 3, 3, 24, 0x00FF_FFFF, "CLIPPED");
        assert!(framebuffer.iter().all(|pixel| *pixel == 0xFF12_3456));

        draw_text(
            &mut framebuffer,
            4,
            4,
            usize::MAX,
            usize::MAX,
            24,
            0xFFFF_FFFF,
            "X",
        );
        assert_eq!(framebuffer.len(), 16);
    }
}

//! Debug overlay rendering with a bitmap font, plus software scanlines fallback.
//!
//! The CRT curvature effect has been moved to a GLSL shader (`gl_render.rs`).

#![allow(
    clippy::erasing_op,
    clippy::identity_op,
    clippy::manual_div_ceil,
    clippy::manual_range_contains,
    clippy::needless_borrow,
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::unnecessary_cast,
    clippy::unused_enumerate_index
)]

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Apply software scanlines to `framebuffer` in place (CPU fallback).
///
/// Not currently used — the GLSL shader handles scanlines on the GPU.
/// Kept as a fallback for environments without OpenGL 3.3 support.
#[allow(dead_code)]
pub fn apply_scanlines(framebuffer: &mut [u32], intensity: f32) {
    scanlines(framebuffer, intensity);
}

/// Draw a debug heads-up display directly onto the framebuffer.
pub fn draw_debug_overlay(
    framebuffer: &mut [u32],
    width: usize,
    fps: f64,
    pc: u32,
    sr: u16,
    cpu_cycles: i32,
    target_cycles: i32,
    rom_label: &str,
) {
    let margin = 4;
    let line_height = 10;

    let mut y = margin;

    // FPS – cyan
    draw_text(
        framebuffer,
        width,
        margin,
        y,
        0xFF00F0FF,
        &format!("{:.1} FPS", fps),
    );
    y += line_height;

    // PC & SR – amber
    draw_text(
        framebuffer,
        width,
        margin,
        y,
        0xFFFFD166,
        &format!("PC:{:06X}  SR:{:04X}", pc, sr),
    );
    y += line_height;

    // CPU cycles – green
    draw_text(
        framebuffer,
        width,
        margin,
        y,
        0xFF9BFF6A,
        &format!("CPU:{}/{} cyc", cpu_cycles, target_cycles),
    );
    y += line_height;

    // ROM label (truncated) – magenta
    let label = if rom_label.len() > 24 {
        &rom_label[..21]
    } else {
        rom_label
    };
    draw_text(framebuffer, width, margin, y, 0xFFFF2A8A, label);
}

// ---------------------------------------------------------------------------
// Scanlines (CPU fallback)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn scanlines(framebuffer: &mut [u32], intensity: f32) {
    const W: usize = 320;
    const H: usize = 224;
    let darken = 1.0 - intensity * 0.55;

    for y in 0..H {
        if y % 2 == 0 {
            continue;
        }

        let row_start = y * W;
        for x in 0..W {
            let pixel = framebuffer[row_start + x];
            let a = (pixel >> 24) & 0xFF;
            let r = (((pixel >> 16) & 0xFF) as f32 * darken) as u32;
            let g = (((pixel >> 8) & 0xFF) as f32 * darken) as u32;
            let b = ((pixel & 0xFF) as f32 * darken) as u32;
            framebuffer[row_start + x] = (a << 24) | (r << 16) | (g << 8) | b;
        }
    }
}

// ---------------------------------------------------------------------------
// Bitmap font (5×7 pixels, monospace for debug overlay)
// ---------------------------------------------------------------------------

/// Each glyph is 5 columns × 7 rows, stored as 5 bytes (bits 6..0 = pixel rows).
type Glyph = [u8; 5];

fn font_glyph(ch: char) -> &'static Glyph {
    let idx = match ch {
        '0'..='9' => ch as usize - '0' as usize,
        'A'..='Z' => 10 + (ch as usize - 'A' as usize),
        'a'..='z' => 10 + (ch as usize - 'a' as usize),
        ' ' => 36,
        ':' => 37,
        '.' => 38,
        '/' => 39,
        '-' => 40,
        '(' => 41,
        ')' => 42,
        '\u{2190}' => 43, // ← left arrow
        '\u{2192}' => 44, // → right arrow
        '\u{2191}' => 45, // ↑ up arrow
        '\u{2193}' => 46, // ↓ down arrow
        '>' => 47,
        '%' => 48,
        '\u{2026}' => 49, // … ellipsis
        '!' => 50,
        '?' => 51,
        '\u{00f1}' => 52, // ñ
        '\u{00d1}' => 52, // Ñ
        '\u{00e1}' => 53, // á
        '\u{00e9}' => 54, // é
        '\u{00ed}' => 55, // í
        '\u{00f3}' => 56, // ó
        '\u{00fa}' => 57, // ú
        '\u{00c1}' => 53, // Á
        '\u{00c9}' => 54, // É
        '\u{00cd}' => 55, // Í
        '\u{00d3}' => 56, // Ó
        '\u{00da}' => 57, // Ú
        '\u{00fc}' => 39, // ü → maps to /
        '\u{00dc}' => 39, // Ü → maps to /
        '\u{2588}' => 58, // █ full block
        '\u{2591}' => 59, // ░ light shade
        _ => 36,          // unknown → space
    };

    if idx < FONT.len() {
        &FONT[idx]
    } else {
        &FONT[36]
    }
}

#[rustfmt::skip]
const FONT: [Glyph; 60] = [
    // 0          1          2          3
    [0x3E,0x45,0x49,0x51,0x3E], [0x00,0x42,0x7F,0x40,0x00], [0x62,0x51,0x49,0x49,0x46], [0x22,0x41,0x49,0x49,0x36],
    // 4          5          6          7
    [0x18,0x14,0x12,0x7F,0x10], [0x27,0x45,0x45,0x45,0x39], [0x3E,0x49,0x49,0x49,0x32], [0x01,0x71,0x09,0x05,0x03],
    // 8          9
    [0x36,0x49,0x49,0x49,0x36], [0x26,0x49,0x49,0x49,0x3E],
    // A(10)      B(11)      C(12)      D(13)
    [0x7C,0x12,0x11,0x12,0x7C], [0x7F,0x49,0x49,0x49,0x36], [0x3E,0x41,0x41,0x41,0x22], [0x7F,0x41,0x41,0x41,0x3E],
    // E(14)      F(15)      G(16)      H(17)
    [0x7F,0x49,0x49,0x49,0x41], [0x7F,0x09,0x09,0x09,0x01], [0x3E,0x41,0x49,0x49,0x3A], [0x7F,0x08,0x08,0x08,0x7F],
    // I(18)      J(19)      K(20)      L(21)
    [0x00,0x41,0x7F,0x41,0x00], [0x20,0x40,0x41,0x41,0x3F], [0x7F,0x08,0x14,0x22,0x41], [0x7F,0x40,0x40,0x40,0x40],
    // M(22)      N(23)      O(24)      P(25)
    [0x7F,0x02,0x04,0x02,0x7F], [0x7F,0x02,0x04,0x08,0x7F], [0x3E,0x41,0x41,0x41,0x3E], [0x7F,0x09,0x09,0x09,0x06],
    // Q(26)      R(27)      S(28)      T(29)
    [0x3E,0x41,0x51,0x21,0x5E], [0x7F,0x09,0x19,0x29,0x46], [0x26,0x49,0x49,0x49,0x32], [0x01,0x01,0x7F,0x01,0x01],
    // U(30)      V(31)      W(32)      X(33)
    [0x3F,0x40,0x40,0x40,0x3F], [0x07,0x08,0x30,0x08,0x07], [0x1F,0x20,0x18,0x20,0x1F], [0x41,0x22,0x1C,0x22,0x41],
    // Y(34)      Z(35)
    [0x03,0x04,0x78,0x04,0x03], [0x61,0x51,0x49,0x45,0x43],
    // space(36)  :(37)      .(38)      /(39)
    [0x00,0x00,0x00,0x00,0x00], [0x00,0x36,0x36,0x00,0x00], [0x00,0x60,0x60,0x00,0x00], [0x40,0x30,0x0C,0x03,0x00],
    // -(40)      ((41)      )(42)      ←(43)
    [0x08,0x08,0x08,0x08,0x08], [0x1C,0x22,0x41,0x00,0x00], [0x41,0x22,0x1C,0x00,0x00], [0x08,0x1C,0x2A,0x08,0x08],
    // →(44)      ↑(45)      ↓(46)      >(47)
    [0x08,0x08,0x2A,0x1C,0x08], [0x08,0x1C,0x3E,0x08,0x08], [0x08,0x08,0x3E,0x1C,0x08], [0x00,0x08,0x1C,0x08,0x00],
    // %(48)      …(49)      !(50)      ?(51)
    [0x42,0x25,0x12,0x48,0x21], [0x00,0x00,0x00,0x00,0x6D], [0x00,0x00,0x5F,0x00,0x00], [0x02,0x01,0x51,0x09,0x06],
    // ñ(52)      á(53)      é(54)      í(55)
    [0x7F,0x02,0x04,0x08,0x7F], [0x7C,0x12,0x11,0x12,0x7C], [0x7F,0x49,0x49,0x49,0x41], [0x00,0x41,0x7F,0x41,0x00],
    // ó(56)      ú(57)
    [0x3E,0x41,0x41,0x41,0x3E], [0x3F,0x40,0x40,0x40,0x3F],
    // █(58)      ░(59)
    [0x7F,0x7F,0x7F,0x7F,0x7F], [0x55,0x2A,0x55,0x2A,0x55],
];

fn draw_text(framebuffer: &mut [u32], fb_width: usize, x: usize, y: usize, color: u32, text: &str) {
    const GLYPH_W: usize = 5;
    const GLYPH_H: usize = 7;
    const CHAR_W: usize = 6; // glyph width + 1 pixel gap
    let mut cursor_x = x;

    for ch in text.chars() {
        let glyph = font_glyph(ch);
        for col in 0..GLYPH_W {
            let col_byte = glyph[col]; // each byte encodes one column of 7 rows
            for row in 0..GLYPH_H {
                if col_byte & (1 << row) != 0 {
                    let px = cursor_x + col;
                    let py = y + row;
                    if px < fb_width {
                        let idx = py * fb_width + px;
                        if idx < framebuffer.len() {
                            framebuffer[idx] = color;
                        }
                    }
                }
            }
        }

        cursor_x += CHAR_W;
    }
}

fn blend_pixel(dst: u32, src: u32, alpha: u8) -> u32 {
    let inv = 255 - alpha as u32;
    let alpha = alpha as u32;
    let r = (((src >> 16) & 0xFF) * alpha + ((dst >> 16) & 0xFF) * inv) / 255;
    let g = (((src >> 8) & 0xFF) * alpha + ((dst >> 8) & 0xFF) * inv) / 255;
    let b = ((src & 0xFF) * alpha + (dst & 0xFF) * inv) / 255;
    0xFF000000 | (r << 16) | (g << 8) | b
}

fn put_pixel_blend(
    framebuffer: &mut [u32],
    width: usize,
    x: usize,
    y: usize,
    color: u32,
    alpha: u8,
) {
    let height = framebuffer.len() / width;
    if x >= width || y >= height {
        return;
    }

    let idx = y * width + x;
    framebuffer[idx] = blend_pixel(framebuffer[idx], color, alpha);
}

fn fill_rect_blend(
    framebuffer: &mut [u32],
    width: usize,
    x: usize,
    y: usize,
    rect_w: usize,
    rect_h: usize,
    color: u32,
    alpha: u8,
) {
    let height = framebuffer.len() / width;
    for row in 0..rect_h {
        let py = y + row;
        if py >= height {
            break;
        }
        for col in 0..rect_w {
            let px = x + col;
            if px >= width {
                break;
            }
            put_pixel_blend(framebuffer, width, px, py, color, alpha);
        }
    }
}

fn draw_rect_outline(
    framebuffer: &mut [u32],
    width: usize,
    x: usize,
    y: usize,
    rect_w: usize,
    rect_h: usize,
    color: u32,
) {
    if rect_w == 0 || rect_h == 0 {
        return;
    }

    for col in 0..rect_w {
        put_pixel_blend(framebuffer, width, x + col, y, color, 255);
        put_pixel_blend(framebuffer, width, x + col, y + rect_h - 1, color, 255);
    }
    for row in 0..rect_h {
        put_pixel_blend(framebuffer, width, x, y + row, color, 255);
        put_pixel_blend(framebuffer, width, x + rect_w - 1, y + row, color, 255);
    }
}

fn draw_trophy_icon(framebuffer: &mut [u32], width: usize, x: usize, y: usize) {
    const GOLD: u32 = 0xFFFFD54A;
    const GOLD_DARK: u32 = 0xFFB46A00;
    const HILITE: u32 = 0xFFFFFFFF;
    const CYAN: u32 = 0xFF00D7FF;
    const TROPHY: [&str; 16] = [
        "....888888....",
        "...88999988...",
        "..8899999988..",
        ".889889988988.",
        ".899889988998.",
        "..9988998899..",
        "...99999999...",
        "....999999....",
        ".....9999.....",
        "......99......",
        ".....9999.....",
        "....999999....",
        "...99999999...",
        "..8888888888..",
        ".88CCCCCCCC88.",
        "..8888888888..",
    ];

    for (row, line) in TROPHY.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let color = match ch {
                '9' => GOLD,
                '8' => GOLD_DARK,
                'C' => CYAN,
                'H' => HILITE,
                _ => continue,
            };
            put_pixel_blend(framebuffer, width, x + col, y + row, color, 255);
        }
    }

    put_pixel_blend(framebuffer, width, x + 5, y + 2, HILITE, 220);
    put_pixel_blend(framebuffer, width, x + 6, y + 2, HILITE, 180);
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }

    let keep = max_chars.saturating_sub(2);
    let mut truncated: String = text.chars().take(keep).collect();
    truncated.push_str("..");
    truncated
}

// ---------------------------------------------------------------------------
// Notification overlay
// ---------------------------------------------------------------------------

/// Draw a notification message centered at the bottom of the screen.
/// Uses the bitmap font with a semi-transparent dark background bar.
pub fn draw_notification(framebuffer: &mut [u32], width: usize, msg: &str) {
    const CHAR_W: usize = 6;
    const CHAR_H: usize = 7;
    const PADDING: usize = 4;

    let text_len = msg.len() * CHAR_W;
    let bar_w = text_len + PADDING * 2;
    let bar_h = CHAR_H + PADDING * 2;
    let height = framebuffer.len() / width;

    // Position: bottom-center
    let bar_x = (width.saturating_sub(bar_w)) / 2;
    let bar_y = height.saturating_sub(bar_h + 8);

    // Draw semi-transparent dark background bar
    for row in 0..bar_h {
        let y = bar_y + row;
        if y >= height {
            break;
        }
        for col in 0..bar_w {
            let x = bar_x + col;
            if x >= width {
                break;
            }
            let idx = y * width + x;
            if idx < framebuffer.len() {
                let pixel = framebuffer[idx];
                // Blend: 50% dark overlay
                let r = ((pixel >> 16) & 0xFF) as u32 * 2 / 5;
                let g = ((pixel >> 8) & 0xFF) as u32 * 2 / 5;
                let b = (pixel & 0xFF) as u32 * 2 / 5;
                framebuffer[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }

    // Draw text in bright white on top of the bar
    draw_text(
        framebuffer,
        width,
        bar_x + PADDING,
        bar_y + PADDING,
        0xFFFFFFFF,
        msg,
    );
}

// ---------------------------------------------------------------------------
// Achievement notification overlay
// ---------------------------------------------------------------------------

/// Draw an achievement unlock notification at the given vertical offset.
///
/// `achievement` — name of the unlocked achievement.
/// `points` — RetroAchievements score points.
/// `y_offset` — vertical pixel position for this notification.
/// `lang` — localised strings.
pub fn draw_achievement_notification(
    framebuffer: &mut [u32],
    width: usize,
    achievement: &str,
    points: u32,
    y_offset: u32,
    lang: &super::lang::Lang,
) {
    const CHAR_W: usize = 6;
    const CHAR_H: usize = 7;
    const CARD_H: usize = 38;
    const ICON_W: usize = 18;
    const LEFT_PAD: usize = 12;
    const TEXT_PAD: usize = 8;
    const RIGHT_PAD: usize = 12;

    let height = framebuffer.len() / width;
    if height == 0 || width == 0 {
        return;
    }

    let available_w = width.saturating_sub(18);
    let card_w = if available_w < 220 {
        available_w.max(1)
    } else {
        available_w.clamp(220, 276)
    };
    let card_y = y_offset as usize;
    let card_x = width.saturating_sub(card_w + 10);
    let gold: u32 = 0xFFFFD54A;
    let gold_dark: u32 = 0xFF9D6100;
    let cyan: u32 = 0xFF00D7FF;
    let white: u32 = 0xFFFFFFFF;
    let soft: u32 = 0xFFB8D7E8;

    // Soft shadow and glow.
    fill_rect_blend(
        framebuffer,
        width,
        card_x + 3,
        card_y + 3,
        card_w,
        CARD_H,
        0xFF000000,
        150,
    );
    draw_rect_outline(
        framebuffer,
        width,
        card_x.saturating_sub(1),
        card_y.saturating_sub(1),
        card_w + 2,
        CARD_H + 2,
        0xFF004E68,
    );

    // Dark glass card with a subtle horizontal gradient and scanline texture.
    for row in 0..CARD_H {
        let y = card_y + row;
        if y >= height {
            break;
        }
        for col in 0..card_w {
            let x = card_x + col;
            if x >= width {
                break;
            }
            let t = (col * 255 / card_w) as u32;
            let r = 4 + t / 30;
            let g = 11 + t / 20;
            let b = 24 + t / 12;
            let base = 0xFF000000 | (r << 16) | (g << 8) | b.min(58);
            let alpha = if row % 2 == 0 { 236 } else { 220 };
            put_pixel_blend(framebuffer, width, x, y, base, alpha);
        }
    }

    // Premium double border and left status stripe.
    draw_rect_outline(framebuffer, width, card_x, card_y, card_w, CARD_H, cyan);
    draw_rect_outline(
        framebuffer,
        width,
        card_x + 2,
        card_y + 2,
        card_w.saturating_sub(4),
        CARD_H.saturating_sub(4),
        gold_dark,
    );
    fill_rect_blend(
        framebuffer,
        width,
        card_x + 4,
        card_y + 4,
        3,
        CARD_H - 8,
        gold,
        255,
    );
    fill_rect_blend(
        framebuffer,
        width,
        card_x + 7,
        card_y + 4,
        1,
        CARD_H - 8,
        cyan,
        210,
    );
    fill_rect_blend(
        framebuffer,
        width,
        card_x + 2,
        card_y + 2,
        card_w - 4,
        1,
        white,
        55,
    );

    let icon_x = card_x + LEFT_PAD;
    let icon_y = card_y + 10;
    draw_trophy_icon(framebuffer, width, icon_x, icon_y);

    let pts_str = format!("{} PTS", points);
    let points_w = pts_str.len() * CHAR_W;
    let text_x = icon_x + ICON_W + TEXT_PAD;
    let text_y = card_y + 8;
    let text_right = card_x + card_w - RIGHT_PAD - points_w.saturating_sub(0);
    let text_cols = text_right.saturating_sub(text_x + 6) / CHAR_W;

    let title = lang.ra_achievement_title.replace(['¡', '!'], "");
    draw_text(framebuffer, width, text_x, text_y, gold, &title);

    let achievement_upper = achievement.to_uppercase();
    let truncated = truncate_chars(&achievement_upper, text_cols.max(8));
    draw_text(
        framebuffer,
        width,
        text_x,
        text_y + CHAR_H + 4,
        white,
        &truncated,
    );
    draw_text(
        framebuffer,
        width,
        text_right,
        text_y + CHAR_H + 4,
        gold,
        &pts_str,
    );

    // Tiny RA monogram in the corner.
    let ra_x = card_x + card_w.saturating_sub(RIGHT_PAD + 12);
    draw_text(framebuffer, width, ra_x, card_y + 6, soft, "RA");
}

// ---------------------------------------------------------------------------
// Slot indicator overlay
// ---------------------------------------------------------------------------

/// Draw a compact slot indicator bar in the top-right corner.
///
/// `current_slot`: the currently selected slot (0..9)
/// `slot_has_data`: which slots have save state data on disk
///
/// Renders as a row of 10 small squares with slot numbers inside.
/// - Current slot: bright cyan background + white number
/// - Occupied slot: filled green/magenta with visible number
/// - Empty slot: hollow/dim outline
pub fn draw_slot_indicator(
    framebuffer: &mut [u32],
    width: usize,
    current_slot: usize,
    slot_has_data: &[bool; 10],
) {
    const SLOT_W: usize = 12;
    const SLOT_H: usize = 14;
    const GAP: usize = 2;
    const PAD_LEFT: usize = 3;
    const PAD_TOP: usize = 3;
    const TOTAL_W: usize = 10 * SLOT_W + 9 * GAP + PAD_LEFT * 2;
    const TOTAL_H: usize = SLOT_H + PAD_TOP * 2;
    const CHAR_W: usize = 6;
    const CHAR_H: usize = 7;
    let height = framebuffer.len() / width;
    let bar_x = width.saturating_sub(TOTAL_W + 4); // 4px from right edge
    let bar_y = 4; // 4px from top edge

    // Draw semi-transparent background bar
    for row in 0..TOTAL_H {
        let y = bar_y + row;
        if y >= height {
            break;
        }
        for col in 0..TOTAL_W {
            let x = bar_x + col;
            if x >= width {
                break;
            }
            let idx = y * width + x;
            if idx < framebuffer.len() {
                let pixel = framebuffer[idx];
                let r = ((pixel >> 16) & 0xFF) as u32 * 2 / 5;
                let g = ((pixel >> 8) & 0xFF) as u32 * 2 / 5;
                let b = (pixel & 0xFF) as u32 * 2 / 5;
                framebuffer[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }

    // Draw each slot square
    for slot in 0..10 {
        let sx = bar_x + PAD_LEFT + slot * (SLOT_W + GAP);
        let sy = bar_y + PAD_TOP;
        let has_data = slot_has_data[slot];
        let is_current = slot == current_slot;

        // Choose colors
        let (fill_color, text_color) = if is_current {
            (0xFF00BBFF, 0xFFFFFFFF) // cyan bg, white text
        } else if has_data {
            (0xFF005500, 0xFF66FF66) // dark green bg, bright green text
        } else {
            (0x00333333, 0xFF666666) // dim bg, gray text
        };

        // Draw filled background for current/occupied, hollow for empty
        for row in 0..SLOT_H {
            let y = sy + row;
            if y >= height {
                break;
            }
            for col in 0..SLOT_W {
                let x = sx + col;
                if x >= width {
                    break;
                }
                let idx = y * width + x;
                if idx >= framebuffer.len() {
                    continue;
                }
                if has_data || is_current {
                    // Filled background
                    let pixel = framebuffer[idx];
                    let r = ((pixel >> 16) & 0xFF) as u32;
                    let g = ((pixel >> 8) & 0xFF) as u32;
                    let b = (pixel & 0xFF) as u32;
                    // Blend fill color over pixel
                    let fr = (fill_color >> 16) & 0xFF;
                    let fg = (fill_color >> 8) & 0xFF;
                    let fb = fill_color & 0xFF;
                    // 70% fill, 30% original
                    let blend_r = (fr * 7 + r * 3) / 10;
                    let blend_g = (fg * 7 + g * 3) / 10;
                    let blend_b = (fb * 7 + b * 3) / 10;
                    framebuffer[idx] = 0xFF000000 | (blend_r << 16) | (blend_g << 8) | blend_b;
                } else {
                    // Empty: draw a border outline
                    let is_border = row == 0 || row == SLOT_H - 1 || col == 0 || col == SLOT_W - 1;
                    if is_border {
                        let pixel = framebuffer[idx];
                        let r = ((pixel >> 16) & 0xFF) as u32 * 3 / 5;
                        let g = ((pixel >> 8) & 0xFF) as u32 * 3 / 5;
                        let b = (pixel & 0xFF) as u32 * 3 / 5;
                        framebuffer[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
                    }
                }
            }
        }

        // Draw the slot number centered in the square
        let num_str = format!("{}", slot);
        let text_x = sx + (SLOT_W.saturating_sub(CHAR_W)) / 2;
        let text_y = sy + (SLOT_H.saturating_sub(CHAR_H)) / 2;
        draw_text(framebuffer, width, text_x, text_y, text_color, &num_str);
    }
}

// ---------------------------------------------------------------------------
// Save State Manager Overlay
// ---------------------------------------------------------------------------

/// Information about one save slot for the manager display.
pub struct SlotInfo {
    /// Whether the slot has saved data.
    pub has_data: bool,
    /// Human-readable file size (e.g. "45.2 KB").
    pub file_size: String,
    /// Human-readable timestamp (e.g. "2025-05-26 14:30").
    pub timestamp: String,
}

/// Draw the full-screen Save State Manager overlay.
///
/// `slots` — array of SlotInfo for each slot (0..9).
/// `selected` — index of the currently highlighted slot.
/// `rom_label` — name of the loaded ROM.
/// `thumbnail` — optional pixel data for the selected slot's thumbnail.
///               Must be in the same format as the framebuffer (AABBGGRR).
///               If `None`, a placeholder is drawn instead.
/// `thumb_w`, `thumb_h` — dimensions of the thumbnail pixel data.
/// `lang` — language strings for localisation.
pub fn draw_save_state_manager(
    framebuffer: &mut [u32],
    width: usize,
    slots: &[SlotInfo; 10],
    selected: usize,
    rom_label: &str,
    thumbnail: Option<&[u32]>,
    thumb_w: usize,
    thumb_h: usize,
    lang: &super::lang::Lang,
) {
    let height = framebuffer.len() / width;
    if height == 0 {
        return;
    }

    const CHAR_W: usize = 6;
    const CHAR_H: usize = 7;
    // Colors
    const ACCENT: u32 = 0xFF00BBFF; // cyan
    const TEXT_HEADING: u32 = 0xFFDDDDDD;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_DIM: u32 = 0xFF555555;
    const TEXT_HIGHLIGHT: u32 = 0xFFFFFFFF;
    const TEXT_DATA: u32 = 0xFF66FF66; // green for slots with data
    const SEPARATOR: u32 = 0xFF333355;

    // Fill entire screen with dimmed background
    for (_i, pixel) in framebuffer.iter_mut().enumerate() {
        let r = ((*pixel >> 16) & 0xFF) as u32 * 15 / 100;
        let g = ((*pixel >> 8) & 0xFF) as u32 * 15 / 100;
        let b = (*pixel & 0xFF) as u32 * 15 / 100;
        *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
    }

    // --- Title bar ---
    let rom_info = format!("  [{}]", rom_label);
    let title_full = format!("{}{}", lang.ssm_title, rom_info);
    draw_text(framebuffer, width, 4, 2, ACCENT, &title_full);

    // Separator line
    let sep_y = 2 + CHAR_H + 2;
    for x in 0..width {
        let idx = sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = 0xFF333355;
        }
    }

    // Available drawing area (after title)
    let content_y = sep_y + 2;
    let content_h = height.saturating_sub(content_y + CHAR_H + 4 + 2); // leave room for action bar + separator

    // --- Layout: Thumbnail on left, Slot list on right ---
    let thumb_area_w: usize = 164; // 160 + small border
    let list_area_x: usize = thumb_area_w + 8;

    // --- Draw thumbnail preview for selected slot ---
    if slots[selected].has_data {
        if let Some(thumb) = thumbnail {
            // Draw a 1px border around the thumbnail
            let border_color = ACCENT;
            let thumb_draw_w = thumb_w.min(thumb_area_w.saturating_sub(2));
            let thumb_draw_h = thumb_h.min(content_h.saturating_sub(2));
            let thumb_x: usize = 6;
            let thumb_y = content_y + 2;

            // Draw border
            for y in 0..thumb_draw_h + 2 {
                let py = thumb_y.saturating_sub(1) + y;
                if py >= height {
                    break;
                }
                for x in 0..thumb_draw_w + 2 {
                    let px = thumb_x.saturating_sub(1) + x;
                    if px >= width {
                        break;
                    }
                    let is_edge =
                        y == 0 || y == thumb_draw_h + 1 || x == 0 || x == thumb_draw_w + 1;
                    if is_edge {
                        let idx = py * width + px;
                        if idx < framebuffer.len() {
                            framebuffer[idx] = border_color;
                        }
                    }
                }
            }

            // Draw thumbnail pixels (nearest-neighbor scaled down, or direct if same size)
            for y in 0..thumb_draw_h {
                let py = thumb_y + y;
                if py >= height {
                    break;
                }
                // Map to source thumbnail coordinates
                let src_y = if thumb_h > 0 {
                    y * thumb_h / thumb_draw_h
                } else {
                    0
                };
                for x in 0..thumb_draw_w {
                    let px = thumb_x + x;
                    if px >= width {
                        break;
                    }
                    let src_x = if thumb_w > 0 {
                        x * thumb_w / thumb_draw_w
                    } else {
                        0
                    };
                    let src_idx = src_y * thumb_w + src_x;
                    if src_idx < thumb.len() {
                        let idx = py * width + px;
                        if idx < framebuffer.len() {
                            framebuffer[idx] = thumb[src_idx];
                        }
                    }
                }
            }
        } else {
            // Placeholder: show "NO THUMBNAIL"
            draw_text(
                framebuffer,
                width,
                6,
                content_y + 10,
                TEXT_DIM,
                lang.ssm_no_thumbnail,
            );
        }
    } else {
        // Slot is empty — show placeholder
        let placeholder = lang.ssm_slot_empty.replacen("{}", &selected.to_string(), 1);
        draw_text(
            framebuffer,
            width,
            6,
            content_y + 10,
            TEXT_DIM,
            &placeholder,
        );
    }

    // --- Draw slot list on the right ---
    let mut list_y = content_y;
    for slot in 0..10 {
        let is_selected = slot == selected;
        let info = &slots[slot];

        // Calculate text
        let slot_label = if is_selected {
            format!("> Slot {}:", slot)
        } else {
            format!("  Slot {}:", slot)
        };

        let status_text = if info.has_data {
            format!("{}  {}", info.file_size, info.timestamp)
        } else {
            lang.slot_empty.to_string()
        };

        let _line = format!("{} {}", slot_label, status_text);

        // Choose colors
        let (label_color, data_color) = if is_selected {
            (TEXT_HIGHLIGHT, TEXT_DATA)
        } else if info.has_data {
            (TEXT_HEADING, TEXT_BODY)
        } else {
            (TEXT_DIM, TEXT_DIM)
        };

        // If selected, draw a highlight bar behind the line
        if is_selected {
            let bar_len = list_area_x
                .wrapping_neg()
                .wrapping_add(width)
                .saturating_sub(4);
            for row in 0..CHAR_H {
                let y = list_y + row;
                if y >= height {
                    break;
                }
                for col in 0..bar_len {
                    let x = list_area_x + col;
                    if x >= width {
                        break;
                    }
                    let idx = y * width + x;
                    if idx < framebuffer.len() {
                        let pixel = framebuffer[idx];
                        let r = ((pixel >> 16) & 0xFF) as u32;
                        let g = ((pixel >> 8) & 0xFF) as u32;
                        let b = (pixel & 0xFF) as u32;
                        // Blend: subtle cyan tint
                        let blend_r = (0x00 * 3 + r * 7) / 10;
                        let blend_g = (0x33 * 3 + g * 7) / 10;
                        let blend_b = (0x55 * 3 + b * 7) / 10;
                        framebuffer[idx] = 0xFF000000 | (blend_r << 16) | (blend_g << 8) | blend_b;
                    }
                }
            }
        }

        // Draw the arrow/highlight indicator
        draw_text(
            framebuffer,
            width,
            list_area_x,
            list_y,
            label_color,
            &slot_label,
        );

        // Draw status text, offset after "Slot X: " text
        if info.has_data {
            let status_x = list_area_x + slot_label.len() * CHAR_W;
            draw_text(
                framebuffer,
                width,
                status_x,
                list_y,
                data_color,
                &status_text,
            );
        }

        list_y += CHAR_H + 1;
    }

    // --- Bottom action bar ---
    let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
    for x in 0..width {
        let idx = bottom_sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    let bottom_y = bottom_sep_y + 2;
    let actions = if slots[selected].has_data {
        lang.ssm_actions_has_data
    } else {
        lang.ssm_actions_empty
    };
    draw_text(framebuffer, width, 4, bottom_y, TEXT_BODY, actions);
}

// ---------------------------------------------------------------------------
// BIOS Selector Overlay
// ---------------------------------------------------------------------------

/// Draw the BIOS selector overlay (full-screen, dimmed background).
///
/// `bios_list` — list of available BIOS labels (from `list_available_bios`).
/// `selected` — index of the currently highlighted BIOS.
/// `current_bios` — the currently active BIOS label.
/// `lang` — language strings for localisation.
pub fn draw_bios_selector(
    framebuffer: &mut [u32],
    width: usize,
    bios_list: &[String],
    selected: usize,
    current_bios: &str,
    lang: &super::lang::Lang,
) {
    let height = framebuffer.len() / width;
    if height == 0 {
        return;
    }

    const CHAR_H: usize = 7;
    const ACCENT: u32 = 0xFF00BBFF;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_DIM: u32 = 0xFF555555;
    const TEXT_WARN: u32 = 0xFFFF6644;
    const TEXT_HIGHLIGHT: u32 = 0xFFFFFFFF;
    const SEPARATOR: u32 = 0xFF333355;
    const HIGHLIGHT_BG: u32 = 0xFF002244;

    if bios_list.is_empty() {
        // Show a helpful message when no BIOS files are found
        for pixel in framebuffer.iter_mut() {
            let r = ((*pixel >> 16) & 0xFF) as u32 * 15 / 100;
            let g = ((*pixel >> 8) & 0xFF) as u32 * 15 / 100;
            let b = (*pixel & 0xFF) as u32 * 15 / 100;
            *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
        }
        draw_text(framebuffer, width, 4, 2, ACCENT, lang.bios_selector_title);
        let sep_y = 2 + CHAR_H + 2;
        for x in 0..width {
            let idx = sep_y * width + x;
            if idx < framebuffer.len() {
                framebuffer[idx] = SEPARATOR;
            }
        }
        let msg_y = sep_y + 10;
        draw_text(framebuffer, width, 6, msg_y, TEXT_WARN, lang.bios_no_files);
        // Bottom action bar
        let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
        for x in 0..width {
            let idx = bottom_sep_y * width + x;
            if idx < framebuffer.len() {
                framebuffer[idx] = SEPARATOR;
            }
        }
        draw_text(
            framebuffer,
            width,
            4,
            bottom_sep_y + 2,
            TEXT_BODY,
            lang.bios_actions_with_data,
        );
        return;
    }

    // Dim the background
    for pixel in framebuffer.iter_mut() {
        let r = ((*pixel >> 16) & 0xFF) as u32 * 15 / 100;
        let g = ((*pixel >> 8) & 0xFF) as u32 * 15 / 100;
        let b = (*pixel & 0xFF) as u32 * 15 / 100;
        *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
    }

    // --- Title ---
    draw_text(framebuffer, width, 4, 2, ACCENT, lang.bios_selector_title);

    // Separator
    let sep_y = 2 + CHAR_H + 2;
    for x in 0..width {
        let idx = sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    // --- Current BIOS info ---
    let current_line = format!("{} {}", lang.bios_current_label, current_bios);
    draw_text(framebuffer, width, 4, sep_y + 2, TEXT_DIM, &current_line);

    let list_y = sep_y + 2 + CHAR_H + 4;

    // --- BIOS list ---
    for (i, label) in bios_list.iter().enumerate() {
        let row_y = list_y + i * (CHAR_H + 1);
        if row_y + CHAR_H >= height.saturating_sub(CHAR_H + 4) {
            break; // Leave room for bottom action bar
        }

        let is_selected = i == selected;
        let is_active = label == current_bios;

        let prefix = if is_selected { "> " } else { "  " };
        let suffix = if is_active {
            lang.bios_active_suffix
        } else {
            ""
        };
        let line = format!("{}{}{}", prefix, label, suffix);

        let (color, bg_color) = if is_selected {
            (TEXT_HIGHLIGHT, Some(HIGHLIGHT_BG))
        } else if is_active {
            (ACCENT, None)
        } else {
            (TEXT_BODY, None)
        };

        // Draw highlight bar behind selected
        if let Some(bg) = bg_color {
            let bar_len = width.saturating_sub(8);
            for row in 0..CHAR_H {
                let y = row_y + row;
                if y >= height {
                    break;
                }
                for col in 0..bar_len {
                    let x = 4 + col;
                    if x >= width {
                        break;
                    }
                    let idx = y * width + x;
                    if idx < framebuffer.len() {
                        let pixel = framebuffer[idx];
                        let r = ((pixel >> 16) & 0xFF) as u32;
                        let g = ((pixel >> 8) & 0xFF) as u32;
                        let b = (pixel & 0xFF) as u32;
                        let br = ((bg >> 16) & 0xFF) as u32;
                        let bg = ((bg >> 8) & 0xFF) as u32;
                        let bb = (bg & 0xFF) as u32;
                        framebuffer[idx] = 0xFF000000
                            | (((br * 7 + r * 3) / 10) << 16)
                            | (((bg * 7 + g * 3) / 10) << 8)
                            | ((bb * 7 + b * 3) / 10);
                    }
                }
            }
        }

        draw_text(framebuffer, width, 4, row_y, color, &line);
    }

    // --- Bottom action bar ---
    let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
    for x in 0..width {
        let idx = bottom_sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    let bottom_y = bottom_sep_y + 2;
    draw_text(
        framebuffer,
        width,
        4,
        bottom_y,
        TEXT_BODY,
        lang.bios_actions_with_data,
    );
}

// ---------------------------------------------------------------------------
// Gamepad Config Overlay
// ---------------------------------------------------------------------------

/// Draw the gamepad button remapping overlay (full-screen, dimmed background).
///
/// `actions` — the 10 NeoGeo actions in display order, followed by system rows.
/// `mapping` — current button mapping (button → action).
/// `selected` — index into `actions` of the currently highlighted action.
/// `listening` — whether we are waiting for the user to press a button.
/// `lang` — localised strings.
/// `has_gamepad` — when `false`, shows a "no gamepad detected" placeholder.
/// `selected_controller` — which controller is being configured (0-based).
/// `total_controllers` — how many controllers are connected.
/// `frame_counter` — monotonic frame count used for the listening-pulse animation.
pub fn draw_gamepad_config(
    framebuffer: &mut [u32],
    width: usize,
    actions: &[core_emulator::EmuAction; 10],
    mapping: &super::gamepad::ControllerMapping,
    selected: usize,
    listening: bool,
    lang: &super::lang::Lang,
    has_gamepad: bool,
    selected_controller: usize,
    total_controllers: usize,
    frame_counter: u64,
) {
    let height = framebuffer.len() / width;
    if height == 0 {
        return;
    }

    const CHAR_H: usize = 7;
    #[allow(dead_code)]
    const CHAR_W: usize = 6;
    const ACCENT: u32 = 0xFF00BBFF;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_DIM: u32 = 0xFF555555;
    const TEXT_HIGHLIGHT: u32 = 0xFFFFFFFF;
    const TEXT_WARN: u32 = 0xFFFF6644;
    const SEPARATOR: u32 = 0xFF333355;
    const HIGHLIGHT_BG: u32 = 0xFF002244;

    // Dim the background
    for pixel in framebuffer.iter_mut() {
        let r = ((*pixel >> 16) & 0xFF) as u32 * 15 / 100;
        let g = ((*pixel >> 8) & 0xFF) as u32 * 15 / 100;
        let b = (*pixel & 0xFF) as u32 * 15 / 100;
        *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
    }

    // --- Title ---
    draw_text(framebuffer, width, 4, 2, ACCENT, lang.gp_title);

    let sep_y = 2 + CHAR_H + 2;
    for x in 0..width {
        let idx = sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    // --- No-gamepad placeholder ---
    if !has_gamepad {
        // Center the "no gamepad" message
        let msg_w = lang.gp_no_gamepad.len() * 6;
        let msg_x = (width.saturating_sub(msg_w)) / 2;
        let msg_y = height / 2 - CHAR_H / 2;
        draw_text(
            framebuffer,
            width,
            msg_x,
            msg_y,
            TEXT_WARN,
            lang.gp_no_gamepad,
        );

        // Bottom action bar
        let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
        for x in 0..width {
            let idx = bottom_sep_y * width + x;
            if idx < framebuffer.len() {
                framebuffer[idx] = SEPARATOR;
            }
        }
        let bottom_y = bottom_sep_y + 2;
        draw_text(
            framebuffer,
            width,
            4,
            bottom_y,
            TEXT_BODY,
            lang.gp_actions_empty,
        );
        return;
    }

    // --- Controller indicator ---
    let ctrl_str = format!(
        "{} {}/{}",
        "Ctrl",
        selected_controller + 1,
        total_controllers
    );
    let ctrl_x = width.saturating_sub(ctrl_str.len() * 6 + 4);
    draw_text(framebuffer, width, ctrl_x, 2, TEXT_BODY, &ctrl_str);

    // --- Column header ---
    let header_y = sep_y + 2;
    // "Action  →  Button"
    let header = format!("{}        {}", lang.gp_actions_label, "Button");
    draw_text(framebuffer, width, 6, header_y, TEXT_DIM, &header);

    let list_start_y = header_y + CHAR_H + 2;

    // --- Action list ---
    for i in 0..super::gamepad::CONFIG_ACTION_COUNT {
        let row_y = list_start_y + i * (CHAR_H + 1);
        if row_y + CHAR_H >= height.saturating_sub(CHAR_H + 4) {
            break;
        }

        let is_selected = i == selected;

        let (action_label, bound_name) = if i < actions.len() {
            let action = actions[i];
            let bound_button = find_first_button_for_action(mapping, action);
            let bound_name = match bound_button {
                Some(btn) => super::gamepad::button_name(btn).to_string(),
                None => "---".to_string(),
            };
            (super::gamepad::action_name(action).to_string(), bound_name)
        } else {
            let system_action = super::gamepad::ALL_SYSTEM_ACTIONS[i - actions.len()];
            (
                super::gamepad::system_action_name(system_action).to_string(),
                super::gamepad::button_chord_name(mapping.system_chord(system_action)),
            )
        };

        // Pad action name for alignment.
        let padded = format!("{:<12}", action_label);
        let line = if is_selected && listening {
            format!("> {}  \u{2192}  {}", padded, lang.gp_listening)
        } else if is_selected {
            format!("> {}  \u{2192}  {}", padded, bound_name)
        } else {
            format!("  {}  \u{2192}  {}", padded, bound_name)
        };

        let color = if is_selected {
            TEXT_HIGHLIGHT
        } else {
            TEXT_BODY
        };

        // Highlight bar for selected
        if is_selected {
            let bar_len = width.saturating_sub(8);
            for row in 0..CHAR_H {
                let y = row_y + row;
                if y >= height {
                    break;
                }
                for col in 0..bar_len {
                    let x = 4 + col;
                    if x >= width {
                        break;
                    }
                    let idx = y * width + x;
                    if idx < framebuffer.len() {
                        let pixel = framebuffer[idx];
                        let r = ((pixel >> 16) & 0xFF) as u32;
                        let g = ((pixel >> 8) & 0xFF) as u32;
                        let b = (pixel & 0xFF) as u32;
                        let br = (HIGHLIGHT_BG >> 16) & 0xFF;
                        let bg = (HIGHLIGHT_BG >> 8) & 0xFF;
                        let bb = HIGHLIGHT_BG & 0xFF;
                        framebuffer[idx] = 0xFF000000
                            | (((br * 7 + r * 3) / 10) << 16)
                            | (((bg * 7 + g * 3) / 10) << 8)
                            | ((bb * 7 + b * 3) / 10);
                    }
                }
            }
        }

        draw_text(framebuffer, width, 4, row_y, color, &line);
    }

    // --- Bottom action bar ---
    let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
    for x in 0..width {
        let idx = bottom_sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    let bottom_y = bottom_sep_y + 2;
    let ctrl_nav = if total_controllers > 1 {
        " \u{2190}\u{2192}:Ctrl "
    } else {
        ""
    };
    let actions_text = if listening {
        format!("\u{2190} Presiona un bot\u{ef}n | Esc:Cancelar{}", ctrl_nav)
    } else {
        format!("{}{}", lang.gp_actions_with_data, ctrl_nav)
    };
    draw_text(framebuffer, width, 4, bottom_y, TEXT_BODY, &actions_text);

    // If listening, show a wave-pulse indicator under the selected action
    if listening {
        let pulse_y = list_start_y + selected * (CHAR_H + 1) + CHAR_H + 2;
        for x in (4..width.saturating_sub(4)).step_by(2) {
            let idx = pulse_y * width + x;
            if idx < framebuffer.len() {
                // Horizontal wave: groups of 4 dots march right-to-left
                let wave = ((x / 4).wrapping_add(frame_counter as usize / 3) % 8) < 4;
                if wave {
                    framebuffer[idx] = TEXT_WARN;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Keyboard Config Overlay
// ---------------------------------------------------------------------------

/// Draw the keyboard remapping overlay (full-screen, dimmed background).
///
/// `actions` — the 10 NeoGeo actions in display order.
/// `mapping` — current keyboard mapping.
/// `selected` — index into `actions` of the currently highlighted action.
/// `listening` — whether we are waiting for the user to press a key.
/// `lang` — localised strings.
/// `frame_counter` — monotonic frame count used for the listening-pulse animation.
pub fn draw_keyboard_config(
    framebuffer: &mut [u32],
    width: usize,
    actions: &[core_emulator::EmuAction; 10],
    mapping: &super::input::KeyboardMapping,
    selected: usize,
    listening: bool,
    lang: &super::lang::Lang,
    frame_counter: u64,
) {
    let height = framebuffer.len() / width;
    if height == 0 {
        return;
    }

    const CHAR_H: usize = 7;
    const ACCENT: u32 = 0xFF00BBFF;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_DIM: u32 = 0xFF555555;
    const TEXT_HIGHLIGHT: u32 = 0xFFFFFFFF;
    const TEXT_WARN: u32 = 0xFFFF6644;
    const SEPARATOR: u32 = 0xFF333355;
    const HIGHLIGHT_BG: u32 = 0xFF002244;

    // Dim the background
    for pixel in framebuffer.iter_mut() {
        let r = ((*pixel >> 16) & 0xFF) as u32 * 15 / 100;
        let g = ((*pixel >> 8) & 0xFF) as u32 * 15 / 100;
        let b = (*pixel & 0xFF) as u32 * 15 / 100;
        *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
    }

    // --- Title ---
    draw_text(framebuffer, width, 4, 2, ACCENT, lang.kb_title);

    let sep_y = 2 + CHAR_H + 2;
    for x in 0..width {
        let idx = sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    // --- Column header ---
    let header_y = sep_y + 2;
    let header = format!("{}        {}", "Action", "Key");
    draw_text(framebuffer, width, 6, header_y, TEXT_DIM, &header);

    let list_start_y = header_y + CHAR_H + 2;

    // --- Action list ---
    for (i, &action) in actions.iter().enumerate() {
        let row_y = list_start_y + i * (CHAR_H + 1);
        if row_y + CHAR_H >= height.saturating_sub(CHAR_H + 4) {
            break;
        }

        let is_selected = i == selected;

        // Find which key maps to this action
        let bound_key = mapping.key_for_action(action);
        let bound_name = match bound_key {
            Some(k) => super::input::keycode_name(k),
            None => "---",
        };

        let action_label = super::gamepad::action_name(action);
        // Pad action name to 8 chars for alignment
        let padded = format!("{:<8}", action_label);
        let line = if is_selected && listening {
            format!("> {}  \u{2192}  {}", padded, lang.kb_listening)
        } else if is_selected {
            format!("> {}  \u{2192}  {}", padded, bound_name)
        } else {
            format!("  {}  \u{2192}  {}", padded, bound_name)
        };

        let color = if is_selected {
            TEXT_HIGHLIGHT
        } else {
            TEXT_BODY
        };

        // Highlight bar for selected
        if is_selected {
            let bar_len = width.saturating_sub(8);
            for row in 0..CHAR_H {
                let y = row_y + row;
                if y >= height {
                    break;
                }
                for col in 0..bar_len {
                    let x = 4 + col;
                    if x >= width {
                        break;
                    }
                    let idx = y * width + x;
                    if idx < framebuffer.len() {
                        let pixel = framebuffer[idx];
                        let r = ((pixel >> 16) & 0xFF) as u32;
                        let g = ((pixel >> 8) & 0xFF) as u32;
                        let b = (pixel & 0xFF) as u32;
                        let br = (HIGHLIGHT_BG >> 16) & 0xFF;
                        let bg = (HIGHLIGHT_BG >> 8) & 0xFF;
                        let bb = HIGHLIGHT_BG & 0xFF;
                        framebuffer[idx] = 0xFF000000
                            | (((br * 7 + r * 3) / 10) << 16)
                            | (((bg * 7 + g * 3) / 10) << 8)
                            | ((bb * 7 + b * 3) / 10);
                    }
                }
            }
        }

        draw_text(framebuffer, width, 4, row_y, color, &line);
    }

    // --- Bottom action bar ---
    let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
    for x in 0..width {
        let idx = bottom_sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    let bottom_y = bottom_sep_y + 2;
    let actions_text = if listening {
        format!("\u{2190} {} | Esc:Cancel", lang.kb_listening)
    } else {
        lang.kb_actions_with_data.to_string()
    };
    draw_text(framebuffer, width, 4, bottom_y, TEXT_BODY, &actions_text);

    // If listening, show a wave-pulse indicator under the selected action
    if listening {
        let pulse_y = list_start_y + selected * (CHAR_H + 1) + CHAR_H + 2;
        for x in (4..width.saturating_sub(4)).step_by(2) {
            let idx = pulse_y * width + x;
            if idx < framebuffer.len() {
                // Horizontal wave: groups of 4 dots march right-to-left
                let wave = ((x / 4).wrapping_add(frame_counter as usize / 3) % 8) < 4;
                if wave {
                    framebuffer[idx] = TEXT_WARN;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Game Profile Config Overlay
// ---------------------------------------------------------------------------

/// Draw the per-game CRT profile config overlay (full-screen, dimmed background).
///
/// `selected` — index of the currently highlighted row (0=scanlines, 1=curvature, 2=bloom).
/// `game_label` — name of the loaded ROM.
/// `lang` — language strings for localisation.
pub fn draw_profile_config(
    framebuffer: &mut [u32],
    width: usize,
    selected: usize,
    scanlines: bool,
    curvature: bool,
    bloom: bool,
    has_per_game_scanlines: bool,
    has_per_game_curvature: bool,
    has_per_game_bloom: bool,
    game_label: &str,
    lang: &super::lang::Lang,
) {
    let height = framebuffer.len() / width;
    if height == 0 {
        return;
    }

    const CHAR_H: usize = 7;
    const ACCENT: u32 = 0xFF00BBFF;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_DIM: u32 = 0xFF555555;
    const TEXT_HIGHLIGHT: u32 = 0xFFFFFFFF;
    const TEXT_ON: u32 = 0xFF66FF66; // green
    const TEXT_OFF: u32 = 0xFFFF6644; // red
    const SEPARATOR: u32 = 0xFF333355;
    const HIGHLIGHT_BG: u32 = 0xFF002244;

    // Dim the background
    for pixel in framebuffer.iter_mut() {
        let r = ((*pixel >> 16) & 0xFF) as u32 * 15 / 100;
        let g = ((*pixel >> 8) & 0xFF) as u32 * 15 / 100;
        let b = (*pixel & 0xFF) as u32 * 15 / 100;
        *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
    }

    // --- Title ---
    draw_text(framebuffer, width, 4, 2, ACCENT, lang.profile_title);

    // Separator
    let sep_y = 2 + CHAR_H + 2;
    for x in 0..width {
        let idx = sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    // --- Game label ---
    let game_line = format!("{} {}", lang.profile_current_game, game_label);
    draw_text(framebuffer, width, 4, sep_y + 2, TEXT_DIM, &game_line);

    let list_y = sep_y + 2 + CHAR_H + 4;

    // --- Settings rows ---
    let settings: [(bool, bool, &str); 3] = [
        (
            scanlines,
            has_per_game_scanlines,
            lang.profile_label_scanlines,
        ),
        (
            curvature,
            has_per_game_curvature,
            lang.profile_label_curvature,
        ),
        (bloom, has_per_game_bloom, lang.profile_label_bloom),
    ];

    for (i, &(value, has_per_game, label)) in settings.iter().enumerate() {
        let row_y = list_y + i * (CHAR_H + 1);
        if row_y + CHAR_H >= height.saturating_sub(CHAR_H + 4) {
            break;
        }

        let is_selected = i == selected;

        let state_text = if value {
            lang.profile_on
        } else {
            lang.profile_off
        };
        let source_text = if has_per_game {
            lang.profile_game_indicator
        } else {
            lang.profile_global_indicator
        };

        // Build line: "> Scanlines    ON  (profile)"
        let prefix = if is_selected { "> " } else { "  " };
        let line = format!("{}{}", prefix, label);

        let (label_color, bg_color) = if is_selected {
            (TEXT_HIGHLIGHT, Some(HIGHLIGHT_BG))
        } else {
            (TEXT_BODY, None)
        };

        let state_color = if value { TEXT_ON } else { TEXT_OFF };

        // Draw highlight bar behind selected row
        if let Some(bg) = bg_color {
            let bar_len = width.saturating_sub(8);
            for row in 0..CHAR_H {
                let y = row_y + row;
                if y >= height {
                    break;
                }
                for col in 0..bar_len {
                    let x = 4 + col;
                    if x >= width {
                        break;
                    }
                    let idx = y * width + x;
                    if idx < framebuffer.len() {
                        let pixel = framebuffer[idx];
                        let r = ((pixel >> 16) & 0xFF) as u32;
                        let g = ((pixel >> 8) & 0xFF) as u32;
                        let b = (pixel & 0xFF) as u32;
                        let br = ((bg >> 16) & 0xFF) as u32;
                        let bg_g = ((bg >> 8) & 0xFF) as u32;
                        let bb = (bg & 0xFF) as u32;
                        framebuffer[idx] = 0xFF000000
                            | (((br * 7 + r * 3) / 10) << 16)
                            | (((bg_g * 7 + g * 3) / 10) << 8)
                            | ((bb * 7 + b * 3) / 10);
                    }
                }
            }
        }

        // Draw label text
        let label_x = 6;
        draw_text(framebuffer, width, label_x, row_y, label_color, &line);

        // Draw state (ON/OFF) aligned to the right of label area
        let state_x = 16 * 6; // ~16 chars from left
        draw_text(framebuffer, width, state_x, row_y, state_color, state_text);

        // Draw source indicator (profile/global) further right
        let source_x = state_x + 6 * 6; // ~6 chars after state
        draw_text(framebuffer, width, source_x, row_y, TEXT_DIM, source_text);
    }

    // --- Bottom action bar ---
    let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
    for x in 0..width {
        let idx = bottom_sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    let bottom_y = bottom_sep_y + 2;
    draw_text(
        framebuffer,
        width,
        4,
        bottom_y,
        TEXT_BODY,
        lang.profile_actions,
    );

    // --- Legend (showing ON/OFF color meaning) ---
    let legend_y = bottom_y + CHAR_H + 2;
    let legend = format!(
        "{} / {}    {} / {}",
        lang.profile_on,
        lang.profile_off,
        lang.profile_game_indicator,
        lang.profile_global_indicator
    );
    draw_text(framebuffer, width, 4, legend_y, TEXT_DIM, &legend);
}

// ---------------------------------------------------------------------------
// Settings Menu Overlay
// ---------------------------------------------------------------------------

/// Draw the full-screen settings menu with tabbed navigation.
///
/// Tabs:
///   0: Video (Scanlines, Curvature, Bloom, Fullscreen, Aspect Ratio, Window Scale)
///   1: Audio (Volume, Mute)
///   2: System (Language, ROM Dumps, Auto-save, BIOS Selector)
///   3: Controls (Gamepad SDL2, Gamepad Config, Keyboard Config)
///   4: Paths (ROMs Dir, BIOS Dir, Media Dir, Screenshots Dir, Saves Dir)
///   5: RetroAchievements status
///
/// Navigation: ← → switch tabs, ↑ ↓ select item, Enter toggle/open.
pub fn draw_settings_menu(
    framebuffer: &mut [u32],
    width: usize,
    selected_tab: usize,
    selected_index: usize,
    scanlines: bool,
    curvature: bool,
    bloom: bool,
    language_is_es: bool,
    diagnostic_dumps: bool,
    fullscreen: bool,
    volume: u8,
    muted: bool,
    window_scale: u32,
    aspect_ratio: &str,
    auto_save: bool,
    gamepad_enabled: bool,
    vol_adjusting: bool,
    ra_logged_in: bool,
    ra_username: &str,
    ra_hardcore: bool,
    ra_has_token: bool,
    ra_has_password: bool,
    ra_game_title: &str,
    ra_game_id: u32,
    ra_achievements: u32,
    ra_unlocked_achievements: u32,
    ra_points_unlocked: u32,
    ra_points_total: u32,
    ra_recent_unlocks: &[(String, u32)],
    ra_game_hash: &str,
    ra_last_status: &str,
    ra_score: u32,
    rom_dir: &str,
    bios_dir: &str,
    media_dir: &str,
    screenshots_dir: &str,
    saves_dir: &str,
    lang: &super::lang::Lang,
) {
    let height = framebuffer.len() / width;
    if height == 0 {
        return;
    }

    const CHAR_H: usize = 7;
    const CHAR_W: usize = 6;
    const ACCENT: u32 = 0xFF00BBFF;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_ON: u32 = 0xFF66FF66;
    const TEXT_OFF: u32 = 0xFFFF6644;
    const SEPARATOR: u32 = 0xFF333355;
    const TAB_BG: u32 = 0xFF001122;

    // Dim the background
    for pixel in framebuffer.iter_mut() {
        let r = ((*pixel >> 16) & 0xFF) as u32 * 15 / 100;
        let g = ((*pixel >> 8) & 0xFF) as u32 * 15 / 100;
        let b = (*pixel & 0xFF) as u32 * 15 / 100;
        *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
    }

    // --- Title ---
    draw_text(framebuffer, width, 4, 2, ACCENT, lang.settings_title);

    // --- Tab bar ---
    let tab_y = 2 + CHAR_H + 2;
    let tabs = [
        lang.settings_tab_video,
        lang.settings_tab_audio,
        lang.settings_tab_system,
        lang.settings_tab_controls,
        lang.settings_tab_paths,
        lang.settings_tab_ra,
    ];
    let tab_gap = 10; // pixels between tabs
    let tab_start_x = 4;
    let mut tab_x = tab_start_x;
    for (i, tab_label) in tabs.iter().enumerate() {
        let is_selected = i == selected_tab;
        let color = if is_selected { ACCENT } else { TEXT_BODY };
        let tab_w = tab_label.len() * CHAR_W;

        // Draw subtle tab background for selected tab
        if is_selected {
            for row in 0..CHAR_H {
                let y = tab_y + row;
                if y >= height {
                    break;
                }
                for col in 0..tab_w {
                    let x = tab_x + col;
                    if x >= width {
                        break;
                    }
                    let idx = y * width + x;
                    if idx < framebuffer.len() {
                        let pixel = framebuffer[idx];
                        let r = ((pixel >> 16) & 0xFF) as u32;
                        let g = ((pixel >> 8) & 0xFF) as u32;
                        let b = (pixel & 0xFF) as u32;
                        let br = (TAB_BG >> 16) & 0xFF;
                        let bg = (TAB_BG >> 8) & 0xFF;
                        let bb = TAB_BG & 0xFF;
                        framebuffer[idx] = 0xFF000000
                            | (((br * 5 + r * 5) / 10) << 16)
                            | (((bg * 5 + g * 5) / 10) << 8)
                            | ((bb * 5 + b * 5) / 10);
                    }
                }
            }
        }

        draw_text(framebuffer, width, tab_x, tab_y, color, tab_label);
        tab_x += tab_w + tab_gap;
    }

    // Separator
    let sep_y = tab_y + CHAR_H + 2;
    for x in 0..width {
        let idx = sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    let list_y = sep_y + 4;

    // --- Tab items ---
    match selected_tab {
        0 => {
            // VIDEO tab: Scanlines, Curvature, Bloom, Fullscreen, Aspect Ratio, Window Scale
            let scale_str = format!("{}x", window_scale);
            let items: [(&str, String, u32); 6] = [
                (
                    lang.settings_label_scanlines,
                    if scanlines {
                        lang.profile_on.to_string()
                    } else {
                        lang.profile_off.to_string()
                    },
                    if scanlines { TEXT_ON } else { TEXT_OFF },
                ),
                (
                    lang.settings_label_curvature,
                    if curvature {
                        lang.profile_on.to_string()
                    } else {
                        lang.profile_off.to_string()
                    },
                    if curvature { TEXT_ON } else { TEXT_OFF },
                ),
                (
                    lang.settings_label_bloom,
                    if bloom {
                        lang.profile_on.to_string()
                    } else {
                        lang.profile_off.to_string()
                    },
                    if bloom { TEXT_ON } else { TEXT_OFF },
                ),
                (
                    lang.settings_label_fullscreen,
                    if fullscreen {
                        lang.profile_on.to_string()
                    } else {
                        lang.profile_off.to_string()
                    },
                    if fullscreen { TEXT_ON } else { TEXT_OFF },
                ),
                (
                    lang.settings_label_aspect_ratio,
                    aspect_ratio.to_string(),
                    ACCENT,
                ),
                (lang.settings_label_window_scale, scale_str, ACCENT),
            ];
            for (i, (label, state_text, state_color)) in items.iter().enumerate() {
                draw_settings_row(
                    framebuffer,
                    width,
                    list_y,
                    i,
                    selected_index,
                    height,
                    label,
                    state_text,
                    *state_color,
                );
            }
        }
        1 => {
            // AUDIO tab: Volume, Mute
            let vol_str = if vol_adjusting {
                // Show a visual bar: [=====    ] 50% or similar
                let bar_len = 10usize;
                let filled = ((volume as usize * bar_len) / 100).min(bar_len);
                let bar: String = (0..bar_len)
                    .map(|i| if i < filled { '\u{2588}' } else { '\u{2591}' })
                    .collect();
                format!("[{}] {}%", bar, volume)
            } else {
                format!("{}%", volume)
            };
            let vol_color = if volume >= 70 {
                TEXT_ON
            } else if volume > 0 {
                0xFFFFFF44
            } else {
                TEXT_OFF
            };
            let mute_str = if muted {
                lang.profile_on
            } else {
                lang.profile_off
            };
            let mute_color = if muted { TEXT_OFF } else { TEXT_ON };
            draw_settings_row(
                framebuffer,
                width,
                list_y,
                0,
                selected_index,
                height,
                lang.settings_label_volume,
                &vol_str,
                vol_color,
            );
            draw_settings_row(
                framebuffer,
                width,
                list_y,
                1,
                selected_index,
                height,
                lang.settings_label_mute,
                &mute_str,
                mute_color,
            );
        }
        2 => {
            // SYSTEM tab: Language, ROM Dumps, Auto-save, BIOS Selector
            let items: [(&str, String, u32); 4] = [
                (
                    lang.settings_label_language,
                    if language_is_es {
                        "ES".to_string()
                    } else {
                        "EN".to_string()
                    },
                    TEXT_ON,
                ),
                (
                    lang.settings_label_dumps,
                    if diagnostic_dumps {
                        "ON".to_string()
                    } else {
                        "OFF".to_string()
                    },
                    if diagnostic_dumps { TEXT_ON } else { TEXT_OFF },
                ),
                (
                    lang.settings_label_auto_save,
                    if auto_save {
                        lang.profile_on.to_string()
                    } else {
                        lang.profile_off.to_string()
                    },
                    if auto_save { TEXT_ON } else { TEXT_OFF },
                ),
                (lang.settings_bios_open, ">".to_string(), ACCENT),
            ];
            for (i, (label, state_text, state_color)) in items.iter().enumerate() {
                draw_settings_row(
                    framebuffer,
                    width,
                    list_y,
                    i,
                    selected_index,
                    height,
                    label,
                    state_text,
                    *state_color,
                );
            }
        }
        3 => {
            // CONTROLS tab: SDL gamepad backend, Gamepad Config, Keyboard Config
            let items: [(&str, String, u32); 3] = [
                (
                    lang.settings_gamepad_enabled,
                    if gamepad_enabled {
                        lang.profile_on.to_string()
                    } else {
                        lang.profile_off.to_string()
                    },
                    if gamepad_enabled { TEXT_ON } else { TEXT_OFF },
                ),
                (lang.settings_gamepad_open, ">".to_string(), ACCENT),
                (lang.settings_keyboard_open, ">".to_string(), ACCENT),
            ];
            for (i, (label, state_text, state_color)) in items.iter().enumerate() {
                draw_settings_row(
                    framebuffer,
                    width,
                    list_y,
                    i,
                    selected_index,
                    height,
                    label,
                    state_text,
                    *state_color,
                );
            }
        }
        4 => {
            // PATHS tab: ROMs, BIOS, Media, Screenshots, Saves
            let truncate = |s: &str, max: usize| -> String {
                if s.len() > max {
                    format!("{}..", &s[..max.saturating_sub(2)])
                } else {
                    s.to_string()
                }
            };
            let items: [(&str, String, u32); 5] = [
                (
                    lang.settings_label_rom_dir,
                    truncate(rom_dir, 30),
                    TEXT_BODY,
                ),
                (
                    lang.settings_label_bios_dir,
                    truncate(bios_dir, 30),
                    TEXT_BODY,
                ),
                (
                    lang.settings_label_media_dir,
                    truncate(media_dir, 30),
                    ACCENT,
                ),
                (
                    lang.settings_label_screenshot_dir,
                    truncate(screenshots_dir, 30),
                    TEXT_BODY,
                ),
                (
                    lang.settings_label_saves_dir,
                    truncate(saves_dir, 30),
                    TEXT_BODY,
                ),
            ];
            for (i, (label, state_text, color)) in items.iter().enumerate() {
                draw_settings_row(
                    framebuffer,
                    width,
                    list_y,
                    i,
                    selected_index,
                    height,
                    label,
                    state_text,
                    *color,
                );
            }
        }
        5 => {
            // RetroAchievements tab: diagnostic/status information.
            let truncate = |s: &str, max: usize| -> String {
                if s.len() > max {
                    format!("{}..", &s[..max.saturating_sub(2)])
                } else {
                    s.to_string()
                }
            };
            let login_state = if ra_logged_in {
                if ra_score > 0 {
                    format!("{} ({} pts)", ra_username, ra_score)
                } else {
                    ra_username.to_string()
                }
            } else {
                lang.ra_not_logged_in.to_string()
            };
            let game_state = if !ra_game_title.is_empty() {
                format!("{} #{}", truncate(ra_game_title, 24), ra_game_id)
            } else {
                lang.ra_game_not_found.to_string()
            };
            let hash_state = if ra_game_hash.len() >= 8 {
                ra_game_hash[..8].to_string()
            } else if ra_game_hash.is_empty() {
                "-".to_string()
            } else {
                ra_game_hash.to_string()
            };
            let credentials_state = if ra_has_token && ra_has_password {
                "TOKEN+PASS"
            } else if ra_has_token {
                "TOKEN"
            } else if ra_has_password {
                "PASSWORD"
            } else {
                lang.profile_off
            };
            let recent_state = if ra_recent_unlocks.is_empty() {
                lang.ra_no_recent_unlocks.to_string()
            } else {
                truncate(
                    &ra_recent_unlocks
                        .iter()
                        .map(|(title, points)| format!("+{} {}", points, title))
                        .collect::<Vec<_>>()
                        .join(" | "),
                    30,
                )
            };
            let items: [(&str, String, u32); 7] = [
                (
                    lang.ra_login,
                    login_state,
                    if ra_logged_in { TEXT_ON } else { TEXT_OFF },
                ),
                (
                    lang.ra_credentials_label,
                    credentials_state.to_string(),
                    if ra_has_token || ra_has_password {
                        TEXT_ON
                    } else {
                        TEXT_OFF
                    },
                ),
                (
                    "Juego RA",
                    game_state,
                    if ra_game_id != 0 { TEXT_ON } else { TEXT_BODY },
                ),
                (
                    lang.ra_progress_label,
                    format!("{}/{}", ra_unlocked_achievements, ra_achievements),
                    ACCENT,
                ),
                (
                    lang.ra_points_label,
                    format!(
                        "{}/{} | {}",
                        ra_points_unlocked, ra_points_total, hash_state
                    ),
                    ACCENT,
                ),
                (lang.ra_recent_unlocks_label, recent_state, TEXT_ON),
                (
                    lang.ra_hardcore_label,
                    format!(
                        "{} | {}",
                        if ra_hardcore {
                            lang.profile_on
                        } else {
                            lang.profile_off
                        },
                        truncate(ra_last_status, 20)
                    ),
                    if ra_hardcore { TEXT_ON } else { TEXT_BODY },
                ),
            ];
            for (i, (label, state_text, state_color)) in items.iter().enumerate() {
                draw_settings_row(
                    framebuffer,
                    width,
                    list_y,
                    i,
                    selected_index,
                    height,
                    label,
                    state_text,
                    *state_color,
                );
            }
        }
        _ => {}
    }

    // --- Bottom action bar ---
    let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
    for x in 0..width {
        let idx = bottom_sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    let bottom_y = bottom_sep_y + 2;
    draw_text(
        framebuffer,
        width,
        4,
        bottom_y,
        TEXT_BODY,
        lang.settings_actions,
    );
}

/// Draw one settings row with label, state text, and optional highlight bar.
/// `state_text` is `&str` to allow both static and dynamic strings.
fn draw_settings_row(
    framebuffer: &mut [u32],
    width: usize,
    list_y: usize,
    index: usize,
    selected_index: usize,
    height: usize,
    label: &str,
    state_text: &str,
    state_color: u32,
) {
    const CHAR_H: usize = 7;
    const CHAR_W: usize = 6;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_HIGHLIGHT: u32 = 0xFFFFFFFF;
    const HIGHLIGHT_BG: u32 = 0xFF002244;

    let row_y = list_y + index * (CHAR_H + 1);
    if row_y + CHAR_H >= height.saturating_sub(CHAR_H + 4) {
        return;
    }

    let is_selected = index == selected_index;
    let prefix = if is_selected { "> " } else { "  " };
    let line = format!("{}{}", prefix, label);
    let label_color = if is_selected {
        TEXT_HIGHLIGHT
    } else {
        TEXT_BODY
    };

    // Draw highlight bar behind selected row
    if is_selected {
        let bar_len = width.saturating_sub(8);
        for row in 0..CHAR_H {
            let y = row_y + row;
            if y >= height {
                break;
            }
            for col in 0..bar_len {
                let x = 4 + col;
                if x >= width {
                    break;
                }
                let idx = y * width + x;
                if idx < framebuffer.len() {
                    let pixel = framebuffer[idx];
                    let r = ((pixel >> 16) & 0xFF) as u32;
                    let g = ((pixel >> 8) & 0xFF) as u32;
                    let b = (pixel & 0xFF) as u32;
                    let br = (HIGHLIGHT_BG >> 16) & 0xFF;
                    let bg = (HIGHLIGHT_BG >> 8) & 0xFF;
                    let bb = HIGHLIGHT_BG & 0xFF;
                    framebuffer[idx] = 0xFF000000
                        | (((br * 7 + r * 3) / 10) << 16)
                        | (((bg * 7 + g * 3) / 10) << 8)
                        | ((bb * 7 + b * 3) / 10);
                }
            }
        }
    }

    // Draw label
    let label_x = 6;
    draw_text(framebuffer, width, label_x, row_y, label_color, &line);

    // Draw state text aligned to right
    let state_x = 26 * CHAR_W; // right-aligned position
    draw_text(framebuffer, width, state_x, row_y, state_color, state_text);
}

// ---------------------------------------------------------------------------
// ROM Browser Overlay
// ---------------------------------------------------------------------------

/// Draw the full-screen ROM browser with a grid of games and box art.
///
/// Displays up to 4 games per page (2 columns × 2 rows).
/// `entries` — the list of all ROM entries.
/// `selected` — index of the currently selected entry.
/// `scroll_offset` — how many entries to skip from the start (page-based).
/// `lang` — language strings.
pub fn draw_rom_browser(
    framebuffer: &mut [u32],
    width: usize,
    entries: &[crate::RomEntry],
    selected: usize,
    scroll_offset: usize,
    lang: &super::lang::Lang,
) {
    let height = framebuffer.len() / width;
    if height == 0 {
        return;
    }

    const CHAR_H: usize = 7;
    const CHAR_W: usize = 6;
    const ACCENT: u32 = 0xFF00BBFF;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_DIM: u32 = 0xFF555555;
    const TEXT_HIGHLIGHT: u32 = 0xFFFFFFFF;
    const SEPARATOR: u32 = 0xFF333355;
    const THUMB_W: usize = 150;
    const THUMB_H: usize = 84;
    const CELL_W: usize = 158;
    const CELL_H: usize = 96;
    const COLS: usize = 2;
    const ROWS: usize = 2;
    const GRID_LEFT: usize = 2;
    const GRID_TOP: usize = 15;

    // Dim the background
    for pixel in framebuffer.iter_mut() {
        let r = ((*pixel >> 16) & 0xFF) as u32 * 15 / 100;
        let g = ((*pixel >> 8) & 0xFF) as u32 * 15 / 100;
        let b = (*pixel & 0xFF) as u32 * 15 / 100;
        *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
    }

    // --- Title ---
    draw_text(framebuffer, width, 4, 2, ACCENT, lang.rb_title);

    // Separator
    let sep_y = 2 + CHAR_H + 2;
    for x in 0..width {
        let idx = sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }

    // --- Page / count indicator ---
    let total = entries.len();
    let per_page = COLS * ROWS;
    let current_page = scroll_offset / per_page;
    let total_pages = ((total + per_page - 1) / per_page).max(1);
    let (page_label, games_label) = if lang.language == super::lang::Language::Es {
        ("PAG", if total == 1 { "JUEGO" } else { "JUEGOS" })
    } else {
        ("PAGE", if total == 1 { "GAME" } else { "GAMES" })
    };
    let page_str = format!(
        "{} {}/{} - {} {}",
        page_label,
        current_page + 1,
        total_pages,
        total,
        games_label
    );
    let page_x = width.saturating_sub(page_str.len() * CHAR_W + 4);
    draw_text(framebuffer, width, page_x, 2, TEXT_DIM, &page_str);

    if entries.is_empty() {
        let message = if lang.language == super::lang::Language::Es {
            "NO HAY ROMS .NEO/.ZIP EN LA CARPETA ROMS"
        } else {
            "NO .NEO/.ZIP ROMS FOUND IN ROMS FOLDER"
        };
        let hint = if lang.language == super::lang::Language::Es {
            "COPIA TUS JUEGOS A ROMS/ Y PULSA CTRL+O"
        } else {
            "COPY YOUR GAMES TO ROMS/ AND PRESS CTRL+O"
        };
        let msg_x = width.saturating_sub(message.len() * CHAR_W) / 2;
        let hint_x = width.saturating_sub(hint.len() * CHAR_W) / 2;
        let msg_y = height.saturating_sub(CHAR_H * 3) / 2;
        draw_text(framebuffer, width, msg_x, msg_y, TEXT_BODY, message);
        draw_text(
            framebuffer,
            width,
            hint_x,
            msg_y + CHAR_H + 4,
            TEXT_DIM,
            hint,
        );
    }

    // --- Grid ---
    let vis_count = per_page.min(total.saturating_sub(scroll_offset));
    for i in 0..vis_count {
        let global_idx = scroll_offset + i;
        if global_idx >= total {
            break;
        }
        let entry = &entries[global_idx];
        let col = i % COLS;
        let row = i / COLS;

        let cell_x = GRID_LEFT + col * CELL_W;
        let cell_y = GRID_TOP + row * CELL_H;
        let is_selected = global_idx == selected;

        // --- Draw thumbnail box ---
        let thumb_x = cell_x + (CELL_W.saturating_sub(THUMB_W)) / 2;
        let thumb_y = cell_y;

        if entry.has_thumbnail {
            if let Some(ref thumb) = entry.thumbnail {
                // Thumbnails are pre-scaled to exactly 150×84. Draw them
                // 1:1 so box art stays sharp and easy to distinguish.
                for ty in 0..THUMB_H {
                    let py = thumb_y + ty;
                    if py >= height {
                        break;
                    }
                    for tx in 0..THUMB_W {
                        let px = thumb_x + tx;
                        if px >= width {
                            break;
                        }
                        let src_idx = ty * THUMB_W + tx;
                        if src_idx < thumb.len() {
                            let idx = py * width + px;
                            if idx < framebuffer.len() {
                                framebuffer[idx] = thumb[src_idx];
                            }
                        }
                    }
                }
            } else {
                // Thumbnail metadata says it exists but not loaded yet
                draw_text(
                    framebuffer,
                    width,
                    thumb_x + 2,
                    thumb_y + THUMB_H / 2 - CHAR_H / 2,
                    TEXT_DIM,
                    "...",
                );
            }
        } else {
            // No thumbnail: draw dark rectangle.
            // Fill the entire thumbnail area with a dark background
            for ty in 0..THUMB_H {
                let py = thumb_y + ty;
                if py >= height {
                    break;
                }
                for tx in 0..THUMB_W {
                    let px = thumb_x + tx;
                    if px >= width {
                        break;
                    }
                    let idx = py * width + px;
                    if idx < framebuffer.len() {
                        // Draw a subtle border (2px) and dark fill
                        let is_border = ty < 2 || ty >= THUMB_H - 2 || tx < 2 || tx >= THUMB_W - 2;
                        if is_border {
                            framebuffer[idx] = 0xFF333355;
                        } else {
                            framebuffer[idx] = 0xFF111122;
                        }
                    }
                }
            }
        }

        if is_selected {
            for ty in 0..THUMB_H {
                let py = thumb_y + ty;
                if py >= height {
                    break;
                }
                for tx in 0..THUMB_W {
                    let px = thumb_x + tx;
                    if px >= width {
                        break;
                    }
                    let is_edge = ty < 2 || ty >= THUMB_H - 2 || tx < 2 || tx >= THUMB_W - 2;
                    if is_edge {
                        let idx = py * width + px;
                        if idx < framebuffer.len() {
                            framebuffer[idx] = ACCENT;
                        }
                    }
                }
            }
        }

        // --- Game name and BIOS overlay inside the thumbnail ---
        let strip_h = if entry.recommended_bios.is_empty() {
            CHAR_H + 4
        } else {
            CHAR_H * 2 + 5
        };
        let strip_y = thumb_y + THUMB_H.saturating_sub(strip_h);
        for sy in 0..strip_h {
            let py = strip_y + sy;
            if py >= height {
                break;
            }
            for sx in 0..THUMB_W {
                let px = thumb_x + sx;
                if px >= width {
                    break;
                }
                let idx = py * width + px;
                if idx < framebuffer.len() {
                    framebuffer[idx] = 0xEE050511;
                }
            }
        }

        let name = &entry.name;
        let max_chars = (THUMB_W - 4) / CHAR_W;
        let display_name = if name.len() > max_chars {
            &name[..max_chars.saturating_sub(2)]
        } else {
            name.as_str()
        };
        let name_y = strip_y + 2;
        let name_color = if is_selected {
            TEXT_HIGHLIGHT
        } else {
            TEXT_BODY
        };
        draw_text(
            framebuffer,
            width,
            thumb_x + 2,
            name_y,
            name_color,
            display_name,
        );

        if name.len() > max_chars {
            let dot_x = thumb_x + 2 + display_name.len() * CHAR_W;
            draw_text(framebuffer, width, dot_x, name_y, name_color, "\u{2026}");
        }

        if !entry.recommended_bios.is_empty() {
            let bios_text = format!("{}{}", lang.rb_bios, entry.recommended_bios);
            let max_bios_chars = (THUMB_W - 4) / CHAR_W;
            let display_bios = if bios_text.len() > max_bios_chars {
                &bios_text[..max_bios_chars.saturating_sub(1)]
            } else {
                bios_text.as_str()
            };
            let bios_y = name_y + CHAR_H + 1;
            if bios_y + CHAR_H <= height {
                draw_text(
                    framebuffer,
                    width,
                    thumb_x + 2,
                    bios_y,
                    TEXT_DIM,
                    display_bios,
                );
            }
        }
    }

    // --- Bottom action bar ---
    let bottom_sep_y = height.saturating_sub(CHAR_H + 2 + 2);
    for x in 0..width {
        let idx = bottom_sep_y * width + x;
        if idx < framebuffer.len() {
            framebuffer[idx] = SEPARATOR;
        }
    }
    let bottom_y = bottom_sep_y + 2;
    draw_text(framebuffer, width, 4, bottom_y, TEXT_BODY, lang.rb_actions);
}

// ---------------------------------------------------------------------------
// Welcome Overlay
// ---------------------------------------------------------------------------

/// Draw a welcome overlay when no ROM is loaded.
///
/// Shows the emulator title and keyboard shortcuts to get started.
pub fn draw_welcome_overlay(framebuffer: &mut [u32], width: usize, lang: &super::lang::Lang) {
    let height = framebuffer.len() / width;
    if height == 0 {
        return;
    }

    const ACCENT: u32 = 0xFF00BBFF;
    const TEXT_BODY: u32 = 0xFFAAAAAA;
    const TEXT_DIM: u32 = 0xFF555555;

    // Dim the background
    for pixel in framebuffer.iter_mut() {
        let r = ((*pixel >> 16) & 0xFF) as u32 * 25 / 100;
        let g = ((*pixel >> 8) & 0xFF) as u32 * 25 / 100;
        let b = (*pixel & 0xFF) as u32 * 25 / 100;
        *pixel = 0xFF000000 | (r << 16) | (g << 8) | b;
    }

    // Title centered
    let title = lang.welcome_title;
    let title_x = (width.saturating_sub(title.len() * 6)) / 2;
    let title_y = height / 3;
    draw_text(framebuffer, width, title_x, title_y, ACCENT, title);

    // Instructions
    let line2 = lang.welcome_line1;
    let line3 = lang.welcome_line2;
    let line4 = lang.welcome_line3;
    let line5 = lang.welcome_line4;

    let line_x = (width.saturating_sub(line2.len() * 6)) / 2;
    let mut line_y = title_y + 14;
    draw_text(framebuffer, width, line_x, line_y, TEXT_BODY, line2);

    line_y += 10;
    let line_x2 = (width.saturating_sub(line3.len() * 6)) / 2;
    draw_text(framebuffer, width, line_x2, line_y, TEXT_BODY, line3);

    line_y += 10;
    let line_x3 = (width.saturating_sub(line4.len() * 6)) / 2;
    draw_text(framebuffer, width, line_x3, line_y, TEXT_BODY, line4);

    line_y += 10;
    let line_x4 = (width.saturating_sub(line5.len() * 6)) / 2;
    draw_text(framebuffer, width, line_x4, line_y, TEXT_BODY, line5);

    // Bottom hint
    let hint_text = if lang.language == super::lang::Language::Es {
        "Presiona cualquier tecla para empezar"
    } else {
        "Press any key to start"
    };
    let hint_x = (width.saturating_sub(hint_text.len() * 6)) / 2;
    let hint_y = height - 20;
    draw_text(framebuffer, width, hint_x, hint_y, TEXT_DIM, hint_text);
}

/// Find the first button in `mapping` that maps to `action`.
fn find_first_button_for_action(
    mapping: &super::gamepad::ControllerMapping,
    action: core_emulator::EmuAction,
) -> Option<sdl2::controller::Button> {
    for (i, &a) in mapping.buttons.iter().enumerate() {
        if a == action {
            return Some(super::gamepad::button_from_index(i));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fb() -> Vec<u32> {
        vec![0xFF102030u32; 320 * 224]
    }

    #[test]
    fn scanlines_darken_odd_rows() {
        let mut fb = make_fb();
        fb[0] = 0xFFFFFFFF;
        fb[320] = 0xFFFFFFFF; // y=1,x=0
        apply_scanlines(&mut fb, 1.0);

        assert_eq!(fb[0], 0xFFFFFFFF);
        let odd = fb[320];
        assert!(odd < 0xFFFFFFFF);
        assert_eq!(odd & 0xFF000000, 0xFF000000);
    }

    #[test]
    fn scanlines_skip_even_rows() {
        let mut fb = vec![0xFFFFFFFFu32; 320 * 224];
        apply_scanlines(&mut fb, 0.5);

        assert_eq!(fb[0], 0xFFFFFFFF); // y=0
        assert_ne!(fb[320], 0xFFFFFFFF); // y=1
        assert_eq!(fb[640], 0xFFFFFFFF); // y=2
    }

    #[test]
    fn debug_overlay_does_not_panic() {
        let mut fb = vec![0xFF000000u32; 320 * 224];
        draw_debug_overlay(
            &mut fb,
            320,
            59.9,
            0x1234,
            0x2700,
            12345,
            200000,
            "mslugx.neo",
        );
    }

    #[test]
    fn font_returns_space_for_unknown() {
        assert_eq!(font_glyph('#'), &FONT[36]);
        assert_eq!(font_glyph('0'), &FONT[0]);
        assert_eq!(font_glyph('F'), &FONT[15]);
    }

    #[test]
    fn font_glyph_columns_encode_rows() {
        // '0' glyph: [0x3E,0x45,0x49,0x51,0x3E]
        // col 2 (0x49 = 0b0100_1001): rows 0,3,6 set
        let glyph = font_glyph('0');
        assert!(glyph[2] & (1 << 0) != 0); // row 0 set
        assert!(glyph[2] & (1 << 3) != 0); // row 3 set
        assert!(glyph[2] & (1 << 6) != 0); // row 6 set
        assert!(glyph[2] & (1 << 1) == 0); // row 1 not set
    }

    #[test]
    fn achievement_notification_modifies_framebuffer() {
        let mut fb = make_fb();
        let lang = crate::lang::Lang::spanish();
        draw_achievement_notification(&mut fb, 320, "Test Achievement", 10, 170, &lang);
        // Verify that pixels in the notification area were modified
        // (gold border pixels should differ from the original fill color)
        let modified = (170 * 320..200 * 320).any(|i| fb[i] != 0xFF102030);
        assert!(
            modified,
            "Achievement notification should modify framebuffer pixels"
        );
    }

    #[test]
    fn achievement_notification_bilingual_titles() {
        let mut fb_es = make_fb();
        let mut fb_en = make_fb();
        let lang_es = crate::lang::Lang::spanish();
        let lang_en = crate::lang::Lang::english();

        draw_achievement_notification(&mut fb_es, 320, "Test", 10, 170, &lang_es);
        draw_achievement_notification(&mut fb_en, 320, "Test", 10, 170, &lang_en);

        // Both should render without panic and modify pixels
        assert_ne!(fb_es[170 * 320 + 160], 0xFF102030);
        assert_ne!(fb_en[170 * 320 + 160], 0xFF102030);
        // The localized titles differ:
        // ES: "¡LOGRO DESBLOQUEADO!" vs EN: "ACHIEVEMENT UNLOCKED!"
        // Verify both produce notification bars by checking the number of modified rows.
        let es_modified: Vec<_> = (170..200)
            .filter(|&y| fb_es[y * 320 + 160] != 0xFF102030)
            .collect();
        let en_modified: Vec<_> = (170..200)
            .filter(|&y| fb_en[y * 320 + 160] != 0xFF102030)
            .collect();
        assert_eq!(
            es_modified.len(),
            en_modified.len(),
            "Both languages should produce same-height notification bars"
        );
    }

    #[test]
    fn achievement_notification_handles_long_name() {
        let mut fb = make_fb();
        let lang = crate::lang::Lang::english();
        let long_name = "This is an incredibly long achievement name that should be truncated in the UI display";
        draw_achievement_notification(&mut fb, 320, long_name, 50, 170, &lang);
        // Should not panic with very long achievement names
        let modified = (170 * 320..180 * 320).any(|i| fb[i] != 0xFF102030);
        assert!(modified);
    }

    #[test]
    fn achievement_notification_stacks_at_different_offsets() {
        let mut fb = make_fb();
        let lang = crate::lang::Lang::english();

        // Draw two stacked notifications at different y offsets
        draw_achievement_notification(&mut fb, 320, "First", 5, 140, &lang);
        draw_achievement_notification(&mut fb, 320, "Second", 10, 170, &lang);

        // Both y ranges should have modified pixels
        let top_modified = (140 * 320..165 * 320).any(|i| fb[i] != 0xFF102030);
        let bottom_modified = (170 * 320..195 * 320).any(|i| fb[i] != 0xFF102030);
        assert!(
            top_modified,
            "Top notification should render at y_offset=140"
        );
        assert!(
            bottom_modified,
            "Bottom notification should render at y_offset=170"
        );
    }

    #[test]
    fn achievement_notification_zero_points() {
        let mut fb = make_fb();
        let lang = crate::lang::Lang::english();
        draw_achievement_notification(&mut fb, 320, "Zero Pointer", 0, 170, &lang);
        // Should render "0 pts" without issues
        let modified = (170 * 320..180 * 320).any(|i| fb[i] != 0xFF102030);
        assert!(modified);
    }

    #[test]
    fn rom_browser_empty_state_renders_message() {
        let mut fb = make_fb();
        let lang = crate::lang::Lang::spanish();
        let entries: Vec<crate::RomEntry> = Vec::new();

        draw_rom_browser(&mut fb, 320, &entries, 0, 0, &lang);

        let modified = fb.iter().any(|&pixel| pixel != 0xFF102030);
        assert!(modified, "empty ROM browser should still draw an overlay");
    }
}

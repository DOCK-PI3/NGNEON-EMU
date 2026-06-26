pub use core_emulator::screenshot::save_framebuffer_bmp;

use std::path::Path;

const THUMBNAIL_BACKGROUND: u32 = 0xFF111122;

/// Load a PNG image and return pixel data in `Vec<u32>` framebuffer format (0xAABBGGRR).
/// The image is scaled proportionally with a sharp Catmull-Rom filter to fit
/// inside `target_w × target_h`, centered.
pub fn load_png_thumbnail<P: AsRef<Path>>(
    path: P,
    target_w: usize,
    target_h: usize,
) -> Result<Vec<u32>, String> {
    let img = image::open(&path)
        .map_err(|e| format!("No se pudo cargar PNG {:?}: {e}", path.as_ref()))?
        .into_rgba8();
    let (src_w, src_h) = img.dimensions();
    let src_w = src_w as usize;
    let src_h = src_h as usize;

    let mut pixels = vec![THUMBNAIL_BACKGROUND; target_w * target_h];
    if src_w == 0 || src_h == 0 || target_w == 0 || target_h == 0 {
        return Ok(pixels);
    }

    let (draw_w, draw_h) = if src_w * target_h > src_h * target_w {
        let draw_w = target_w;
        let draw_h = (src_h * target_w / src_w).max(1).min(target_h);
        (draw_w, draw_h)
    } else {
        let draw_h = target_h;
        let draw_w = (src_w * target_h / src_h).max(1).min(target_w);
        (draw_w, draw_h)
    };
    let offset_x = (target_w - draw_w) / 2;
    let offset_y = (target_h - draw_h) / 2;
    let resized = image::imageops::resize(
        &img,
        draw_w as u32,
        draw_h as u32,
        image::imageops::FilterType::CatmullRom,
    );
    let raw = resized.as_raw();

    for dy in 0..draw_h {
        for dx in 0..draw_w {
            let src_idx = (dy * draw_w + dx) * 4;
            let r = raw[src_idx] as u32;
            let g = raw[src_idx + 1] as u32;
            let b = raw[src_idx + 2] as u32;
            let a = raw[src_idx + 3] as u32;
            let bg_r = 0x22;
            let bg_g = 0x11;
            let bg_b = 0x11;
            let out_r = (r * a + bg_r * (255 - a)) / 255;
            let out_g = (g * a + bg_g * (255 - a)) / 255;
            let out_b = (b * a + bg_b * (255 - a)) / 255;
            let dst_idx = (offset_y + dy) * target_w + offset_x + dx;
            pixels[dst_idx] = 0xFF000000 | (out_b << 16) | (out_g << 8) | out_r;
        }
    }
    Ok(pixels)
}

/// Load a 32-bit BGRA BMP file and return pixel data in `[u32]` RGBA format
/// (same byte order as the emulator framebuffer: 0xAABBGGRR in native u32).
pub fn load_framebuffer_bmp<P: AsRef<Path>>(path: P) -> Result<Vec<u32>, String> {
    let data = std::fs::read(&path)
        .map_err(|e| format!("No se pudo leer BMP {:?}: {e}", path.as_ref()))?;
    if data.len() < 54 || &data[0..2] != b"BM" {
        return Err(format!("No es un BMP válido: {:?}", path.as_ref()));
    }
    let pixel_offset = u32::from_le_bytes(data[10..14].try_into().unwrap()) as usize;
    let width = i32::from_le_bytes(data[18..22].try_into().unwrap());
    let height = i32::from_le_bytes(data[22..26].try_into().unwrap());
    let bpp = u16::from_le_bytes(data[28..30].try_into().unwrap());
    let compression = u32::from_le_bytes(data[30..34].try_into().unwrap());

    if width <= 0 || height == 0 || (bpp != 32 && bpp != 24) || compression != 0 {
        return Err(format!(
            "BMP no soportado: {}x{}x{}bpp compression={}",
            width, height, bpp, compression
        ));
    }

    let w = width as usize;
    let h = height as usize;
    let abs_height = h;
    let row_size = if bpp == 32 { w * 4 } else { w * 3 };
    // BMP rows are padded to 4-byte boundaries
    let row_padded = (row_size + 3) & !3;
    let mut pixels = vec![0xFF000000u32; w * abs_height];

    for y in 0..abs_height {
        // BMP stores rows bottom-to-top
        let src_y = abs_height - 1 - y;
        let src_row_start = pixel_offset + src_y * row_padded;
        let dst_row_start = y * w;

        for x in 0..w {
            let src_pixel = src_row_start + x * (bpp as usize / 8);
            if src_pixel + 2 >= data.len() {
                return Err("BMP truncado".to_string());
            }
            let b = data[src_pixel] as u32;
            let g = data[src_pixel + 1] as u32;
            let r = data[src_pixel + 2] as u32;
            let a = if bpp == 32 {
                data.get(src_pixel + 3).copied().unwrap_or(0xFF) as u32
            } else {
                0xFF
            };
            // Store as AABBGGRR (same as emulator framebuffer)
            pixels[dst_row_start + x] = (a << 24) | (r << 16) | (g << 8) | b;
        }
    }

    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn png_thumbnail_fits_entire_image_without_cropping() {
        let path =
            std::env::temp_dir().join(format!("ngneon-thumb-fit-{}.png", std::process::id()));
        let mut image = RgbaImage::new(200, 50);
        for y in 0..50 {
            for x in 0..200 {
                image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        image.save(&path).unwrap();

        let pixels = load_png_thumbnail(&path, 100, 100).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(pixels[0], 0xFF111122);
        assert_eq!(pixels[36 * 100], 0xFF111122);
        assert_eq!(pixels[37 * 100], 0xFF0000FF);
        assert_eq!(pixels[61 * 100 + 99], 0xFF0000FF);
        assert_eq!(pixels[62 * 100], 0xFF111122);
    }

    #[test]
    fn png_thumbnail_uses_sharp_downscaling() {
        let path =
            std::env::temp_dir().join(format!("ngneon-thumb-quality-{}.png", std::process::id()));
        let mut image = RgbaImage::new(256, 256);
        for y in 0..256 {
            for x in 0..256 {
                let value = if (x + y) % 2 == 0 { 255 } else { 0 };
                image.put_pixel(x, y, Rgba([value, value, value, 255]));
            }
        }
        image.save(&path).unwrap();

        let pixels = load_png_thumbnail(&path, 32, 32).unwrap();
        let _ = std::fs::remove_file(&path);

        let center = pixels[16 * 32 + 16];
        let gray = center & 0xFF;
        assert!(
            (96..=160).contains(&gray),
            "Catmull-Rom downscale should preserve averaged detail, got {gray}"
        );
    }
}

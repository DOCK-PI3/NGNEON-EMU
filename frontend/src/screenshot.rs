pub use core_emulator::screenshot::save_framebuffer_bmp;

use std::path::Path;

const THUMBNAIL_BACKGROUND: u32 = 0xFF111122;

/// Load an image in `0xAARRGGBB` format, preserving its aspect ratio and
/// bounding its dimensions to keep UI artwork caches predictable.
pub fn load_image_argb_bounded<P: AsRef<Path>>(
    path: P,
    max_width: usize,
    max_height: usize,
) -> Result<(usize, usize, Vec<u32>), String> {
    let image = load_rgba_image(path.as_ref())?;
    let (source_width, source_height) = image.dimensions();
    if max_width == 0 || max_height == 0 || source_width == 0 || source_height == 0 {
        return Ok((0, 0, Vec::new()));
    }
    let (width, height) = contain_dimensions(source_width, source_height, max_width, max_height)?;
    let image = if width == source_width && height == source_height {
        image
    } else {
        image::imageops::resize(
            &image,
            width,
            height,
            image::imageops::FilterType::CatmullRom,
        )
    };
    let width = width as usize;
    let height = height as usize;
    let pixels = image.as_raw().chunks_exact(4).map(pack_rgba).collect();
    Ok((width, height, pixels))
}

/// Load an image and return pixel data in `Vec<u32>` framebuffer format (0xAARRGGBB).
/// The image is scaled proportionally with a sharp Catmull-Rom filter to fit
/// inside `target_w × target_h`, centered over the legacy thumbnail background.
pub fn load_image_thumbnail<P: AsRef<Path>>(
    path: P,
    target_w: usize,
    target_h: usize,
) -> Result<Vec<u32>, String> {
    load_image_contain(path, target_w, target_h).map(|pixels| {
        pixels
            .into_iter()
            .map(|pixel| composite_over_opaque(pixel, THUMBNAIL_BACKGROUND))
            .collect()
    })
}

/// Load an image scaled proportionally to fit entirely inside the requested size.
///
/// The result is centered on a transparent background and uses straight-alpha
/// `0xAARRGGBB` pixels, suitable for later composition by the caller.
pub fn load_image_contain<P: AsRef<Path>>(
    path: P,
    target_w: usize,
    target_h: usize,
) -> Result<Vec<u32>, String> {
    let img = load_rgba_image(path.as_ref())?;
    let mut pixels = transparent_pixel_buffer(target_w, target_h)?;
    if target_w == 0 || target_h == 0 {
        return Ok(pixels);
    }

    let (src_w, src_h) = img.dimensions();
    if src_w == 0 || src_h == 0 {
        return Ok(pixels);
    }

    let (draw_w, draw_h) = contain_dimensions(src_w, src_h, target_w, target_h)?;
    let resized = image::imageops::resize(
        &img,
        draw_w,
        draw_h,
        image::imageops::FilterType::CatmullRom,
    );
    let draw_w = draw_w as usize;
    let draw_h = draw_h as usize;
    let offset_x = (target_w - draw_w) / 2;
    let offset_y = (target_h - draw_h) / 2;
    let raw = resized.as_raw();

    for dy in 0..draw_h {
        for dx in 0..draw_w {
            let src_idx = (dy * draw_w + dx) * 4;
            let dst_idx = (offset_y + dy) * target_w + offset_x + dx;
            pixels[dst_idx] = pack_rgba(&raw[src_idx..src_idx + 4]);
        }
    }
    Ok(pixels)
}

/// Load an image scaled proportionally to fill the requested size.
///
/// Any overflow is cropped equally from opposing edges, keeping the source
/// centered. Pixels use straight-alpha `0xAARRGGBB` format.
pub fn load_image_cover<P: AsRef<Path>>(
    path: P,
    target_w: usize,
    target_h: usize,
) -> Result<Vec<u32>, String> {
    let img = load_rgba_image(path.as_ref())?;
    let mut pixels = transparent_pixel_buffer(target_w, target_h)?;
    if target_w == 0 || target_h == 0 {
        return Ok(pixels);
    }

    let (src_w, src_h) = img.dimensions();
    if src_w == 0 || src_h == 0 {
        return Ok(pixels);
    }

    let (draw_w, draw_h) = cover_dimensions(src_w, src_h, target_w, target_h)?;
    let resized = image::imageops::resize(
        &img,
        draw_w,
        draw_h,
        image::imageops::FilterType::CatmullRom,
    );
    let draw_w = draw_w as usize;
    let draw_h = draw_h as usize;
    let offset_x = (draw_w - target_w) / 2;
    let offset_y = (draw_h - target_h) / 2;
    let raw = resized.as_raw();

    for dy in 0..target_h {
        for dx in 0..target_w {
            let src_idx = ((offset_y + dy) * draw_w + offset_x + dx) * 4;
            let dst_idx = dy * target_w + dx;
            pixels[dst_idx] = pack_rgba(&raw[src_idx..src_idx + 4]);
        }
    }
    Ok(pixels)
}

/// Load an image resized exactly to the requested size.
///
/// Unlike [`load_image_contain`] and [`load_image_cover`], this mode does not
/// preserve the source aspect ratio. Pixels use straight-alpha `0xAARRGGBB`
/// format.
pub fn load_image_stretch<P: AsRef<Path>>(
    path: P,
    target_w: usize,
    target_h: usize,
) -> Result<Vec<u32>, String> {
    let img = load_rgba_image(path.as_ref())?;
    let mut pixels = transparent_pixel_buffer(target_w, target_h)?;
    if target_w == 0 || target_h == 0 {
        return Ok(pixels);
    }

    let target_w =
        u32::try_from(target_w).map_err(|_| "El ancho de destino no cabe en u32".to_string())?;
    let target_h =
        u32::try_from(target_h).map_err(|_| "El alto de destino no cabe en u32".to_string())?;
    let resized = image::imageops::resize(
        &img,
        target_w,
        target_h,
        image::imageops::FilterType::CatmullRom,
    );

    for (pixel, rgba) in pixels.iter_mut().zip(resized.as_raw().chunks_exact(4)) {
        *pixel = pack_rgba(rgba);
    }
    Ok(pixels)
}

fn load_rgba_image(path: &Path) -> Result<image::RgbaImage, String> {
    image::open(path)
        .map_err(|error| format!("No se pudo cargar la imagen {path:?}: {error}"))
        .map(image::DynamicImage::into_rgba8)
}

fn transparent_pixel_buffer(target_w: usize, target_h: usize) -> Result<Vec<u32>, String> {
    let pixel_count = target_w
        .checked_mul(target_h)
        .ok_or_else(|| format!("Dimensiones de imagen demasiado grandes: {target_w}x{target_h}"))?;
    Ok(vec![0; pixel_count])
}

fn contain_dimensions(
    src_w: u32,
    src_h: u32,
    target_w: usize,
    target_h: usize,
) -> Result<(u32, u32), String> {
    let target_w =
        u32::try_from(target_w).map_err(|_| "El ancho de destino no cabe en u32".to_string())?;
    let target_h =
        u32::try_from(target_h).map_err(|_| "El alto de destino no cabe en u32".to_string())?;

    if u64::from(src_w) * u64::from(target_h) > u64::from(src_h) * u64::from(target_w) {
        let draw_h = (u64::from(src_h) * u64::from(target_w) / u64::from(src_w))
            .max(1)
            .min(u64::from(target_h)) as u32;
        Ok((target_w, draw_h))
    } else {
        let draw_w = (u64::from(src_w) * u64::from(target_h) / u64::from(src_h))
            .max(1)
            .min(u64::from(target_w)) as u32;
        Ok((draw_w, target_h))
    }
}

fn cover_dimensions(
    src_w: u32,
    src_h: u32,
    target_w: usize,
    target_h: usize,
) -> Result<(u32, u32), String> {
    let target_w =
        u32::try_from(target_w).map_err(|_| "El ancho de destino no cabe en u32".to_string())?;
    let target_h =
        u32::try_from(target_h).map_err(|_| "El alto de destino no cabe en u32".to_string())?;

    if u64::from(src_w) * u64::from(target_h) > u64::from(src_h) * u64::from(target_w) {
        let draw_w = div_ceil_u64(u64::from(src_w) * u64::from(target_h), u64::from(src_h));
        let draw_w = u32::try_from(draw_w)
            .map_err(|_| "La imagen escalada excede el ancho soportado".to_string())?;
        Ok((draw_w.max(target_w), target_h))
    } else {
        let draw_h = div_ceil_u64(u64::from(src_h) * u64::from(target_w), u64::from(src_w));
        let draw_h = u32::try_from(draw_h)
            .map_err(|_| "La imagen escalada excede el alto soportado".to_string())?;
        Ok((target_w, draw_h.max(target_h)))
    }
}

fn div_ceil_u64(numerator: u64, denominator: u64) -> u64 {
    numerator / denominator + u64::from(!numerator.is_multiple_of(denominator))
}

fn pack_rgba(rgba: &[u8]) -> u32 {
    (u32::from(rgba[3]) << 24)
        | (u32::from(rgba[0]) << 16)
        | (u32::from(rgba[1]) << 8)
        | u32::from(rgba[2])
}

fn composite_over_opaque(source: u32, background: u32) -> u32 {
    let alpha = (source >> 24) & 0xFF;
    let inverse_alpha = 255 - alpha;
    let red = (((source >> 16) & 0xFF) * alpha + ((background >> 16) & 0xFF) * inverse_alpha) / 255;
    let green = (((source >> 8) & 0xFF) * alpha + ((background >> 8) & 0xFF) * inverse_alpha) / 255;
    let blue = ((source & 0xFF) * alpha + (background & 0xFF) * inverse_alpha) / 255;
    0xFF000000 | (red << 16) | (green << 8) | blue
}

/// Backward-compatible helper for the in-emulator ROM browser.
pub fn load_png_thumbnail<P: AsRef<Path>>(
    path: P,
    target_w: usize,
    target_h: usize,
) -> Result<Vec<u32>, String> {
    load_image_thumbnail(path, target_w, target_h)
}

/// Load a 32-bit BGRA BMP file and return pixels in `0xAARRGGBB` format.
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
            // Store as 0xAARRGGBB.
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
        assert_eq!(pixels[37 * 100], 0xFFFF0000);
        assert_eq!(pixels[61 * 100 + 99], 0xFFFF0000);
        assert_eq!(pixels[62 * 100], 0xFF111122);
    }

    #[test]
    fn bounded_argb_load_preserves_source_aspect_ratio() {
        let path = std::env::temp_dir().join(format!("ngneon-bounded-{}.png", std::process::id()));
        let mut image = RgbaImage::new(400, 100);
        for pixel in image.pixels_mut() {
            *pixel = Rgba([0x11, 0x22, 0x33, 0x44]);
        }
        image.save(&path).unwrap();

        let (width, height, pixels) = load_image_argb_bounded(&path, 100, 100).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!((width, height), (100, 25));
        assert_eq!(pixels.len(), width * height);
    }

    #[test]
    fn contain_preserves_argb_channels_and_transparent_bars() {
        let path =
            std::env::temp_dir().join(format!("ngneon-contain-alpha-{}.png", std::process::id()));
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([0x12, 0x34, 0x56, 0x78]));
        image.save(&path).unwrap();

        let pixels = load_image_contain(&path, 3, 1).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(pixels, vec![0x00000000, 0x78123456, 0x00000000]);
    }

    #[test]
    fn cover_crops_the_scaled_image_from_the_center() {
        let path =
            std::env::temp_dir().join(format!("ngneon-cover-center-{}.png", std::process::id()));
        let mut image = RgbaImage::new(4, 2);
        let columns = [
            Rgba([255, 0, 0, 255]),
            Rgba([0, 255, 0, 255]),
            Rgba([0, 0, 255, 255]),
            Rgba([255, 255, 255, 255]),
        ];
        for y in 0..2 {
            for (x, color) in columns.iter().enumerate() {
                image.put_pixel(x as u32, y, *color);
            }
        }
        image.save(&path).unwrap();

        let pixels = load_image_cover(&path, 2, 2).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(pixels, vec![0xFF00FF00, 0xFF0000FF, 0xFF00FF00, 0xFF0000FF]);
    }

    #[test]
    fn stretch_uses_exact_dimensions_and_preserves_argb_channels() {
        let path =
            std::env::temp_dir().join(format!("ngneon-stretch-alpha-{}.png", std::process::id()));
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([0x12, 0x34, 0x56, 0x78]));
        image.save(&path).unwrap();

        let pixels = load_image_stretch(&path, 3, 2).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(pixels.len(), 3 * 2);
        assert_eq!(pixels, vec![0x78123456; 3 * 2]);
    }

    #[test]
    fn stretch_accepts_zero_target_dimensions() {
        let path =
            std::env::temp_dir().join(format!("ngneon-stretch-zero-{}.png", std::process::id()));
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([0x12, 0x34, 0x56, 0x78]));
        image.save(&path).unwrap();

        let zero_width = load_image_stretch(&path, 0, 3).unwrap();
        let zero_height = load_image_stretch(&path, 3, 0).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(zero_width.is_empty());
        assert!(zero_height.is_empty());
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

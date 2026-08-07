//! CPU-side visual effects and animation transforms for frontend themes.
//!
//! Static layers and selected-game media are processed once and cached. The
//! same functions are also used for rendered components, keeping XML effects
//! independent from SDL/OpenGL and deterministic in screenshots and tests.

use crate::frontend_theme::{
    ThemeAnimation, ThemeGradientDirection, ThemeImageEffects, ThemeRect, ThemeSlideDirection,
};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnimatedVisual {
    pub(crate) rect: ThemeRect,
    pub(crate) opacity: u8,
}

pub(crate) fn apply_image_effects(
    source: &[u32],
    width: usize,
    height: usize,
    effects: ThemeImageEffects,
) -> Vec<u32> {
    let expected = width.saturating_mul(height);
    if width == 0 || height == 0 || source.len() < expected {
        return Vec::new();
    }
    if effects.is_neutral() {
        return source[..expected].to_vec();
    }

    let mut image = source[..expected].to_vec();
    apply_color_adjustments(&mut image, effects);
    if effects.blur > 0 {
        image = box_blur_argb(&image, width, height, usize::from(effects.blur));
    }
    if effects.rotation != 0 {
        image = rotate_argb(&image, width, height, effects.rotation);
    }
    if effects.rounded_corners > 0 {
        apply_rounded_corners(
            &mut image,
            width,
            height,
            usize::from(effects.rounded_corners),
        );
    }
    if let Some(gradient) = effects.gradient {
        apply_gradient_overlay(
            &mut image,
            width,
            height,
            gradient.start,
            gradient.end,
            gradient.direction,
        );
    }

    if effects.shadow.is_none() && effects.glow.is_none() {
        return image;
    }

    let alpha: Vec<u8> = image.iter().map(|pixel| (pixel >> 24) as u8).collect();
    let mut output = vec![0_u32; expected];
    if let Some(glow) = effects.glow.filter(|glow| glow.color >> 24 != 0) {
        let mask = if glow.radius == 0 {
            alpha.clone()
        } else {
            box_blur_mask(&alpha, width, height, usize::from(glow.radius))
        };
        composite_colored_mask(&mut output, width, height, &mask, 0, 0, glow.color);
    }
    if let Some(shadow) = effects.shadow.filter(|shadow| shadow.color >> 24 != 0) {
        let mask = if shadow.blur == 0 {
            alpha
        } else {
            box_blur_mask(&alpha, width, height, usize::from(shadow.blur))
        };
        composite_colored_mask(
            &mut output,
            width,
            height,
            &mask,
            shadow.x,
            shadow.y,
            shadow.color,
        );
    }
    for (destination, source) in output.iter_mut().zip(image) {
        *destination = source_over(source, *destination);
    }
    output
}

pub(crate) fn animate_visual(
    rect: ThemeRect,
    opacity: u8,
    animation: ThemeAnimation,
    elapsed: Duration,
) -> AnimatedVisual {
    if animation.is_static() {
        return AnimatedVisual { rect, opacity };
    }

    let elapsed_ms = elapsed.as_secs_f32() * 1_000.0;
    let mut animated = rect;
    let mut opacity_factor = 1.0_f32;

    if animation.fade_in_ms > 0 {
        opacity_factor *= progress(elapsed_ms, animation.fade_in_ms);
    }

    if !matches!(animation.slide_in, ThemeSlideDirection::None) {
        let remaining = 1.0 - eased_progress(elapsed_ms, animation.slide_duration_ms);
        let offset = (f32::from(animation.slide_distance) * remaining).round() as i32;
        match animation.slide_in {
            ThemeSlideDirection::Left => animated.x = animated.x.saturating_sub(offset),
            ThemeSlideDirection::Right => animated.x = animated.x.saturating_add(offset),
            ThemeSlideDirection::Up => animated.y = animated.y.saturating_sub(offset),
            ThemeSlideDirection::Down => animated.y = animated.y.saturating_add(offset),
            ThemeSlideDirection::None => {}
        }
    }

    let mut scale = 1.0_f32;
    if animation.zoom_in_ms > 0 {
        let from = f32::from(animation.zoom_from) / 100.0;
        scale *= from + (1.0 - from) * eased_progress(elapsed_ms, animation.zoom_in_ms);
    }
    if animation.pulse > 0 && animation.pulse_period_ms > 0 {
        let phase = elapsed.as_secs_f32() * std::f32::consts::TAU * 1_000.0
            / animation.pulse_period_ms as f32;
        let wave = (phase.sin() + 1.0) * 0.5;
        scale *= 1.0 + f32::from(animation.pulse) / 100.0 * wave;
    }
    if (scale - 1.0).abs() > f32::EPSILON {
        animated = scale_rect_about_center(animated, scale);
    }

    AnimatedVisual {
        rect: animated,
        opacity: (f32::from(opacity) * opacity_factor)
            .round()
            .clamp(0.0, 255.0) as u8,
    }
}

pub(crate) fn animation_needs_redraw(animation: ThemeAnimation, elapsed: Duration) -> bool {
    if animation.pulse > 0 {
        return true;
    }
    let elapsed_ms = elapsed.as_millis();
    elapsed_ms < u128::from(animation.fade_in_ms)
        || (!matches!(animation.slide_in, ThemeSlideDirection::None)
            && elapsed_ms < u128::from(animation.slide_duration_ms))
        || elapsed_ms < u128::from(animation.zoom_in_ms)
}

fn progress(elapsed_ms: f32, duration_ms: u32) -> f32 {
    if duration_ms == 0 {
        1.0
    } else {
        (elapsed_ms / duration_ms as f32).clamp(0.0, 1.0)
    }
}

fn eased_progress(elapsed_ms: f32, duration_ms: u32) -> f32 {
    let linear = progress(elapsed_ms, duration_ms);
    1.0 - (1.0 - linear).powi(3)
}

fn scale_rect_about_center(rect: ThemeRect, scale: f32) -> ThemeRect {
    let width = ((rect.width as f32 * scale).round() as u32).max(1);
    let height = ((rect.height as f32 * scale).round() as u32).max(1);
    let delta_x = (i64::from(width) - i64::from(rect.width)) / 2;
    let delta_y = (i64::from(height) - i64::from(rect.height)) / 2;
    ThemeRect::new(
        (i64::from(rect.x) - delta_x).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        (i64::from(rect.y) - delta_y).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        width,
        height,
    )
}

fn apply_color_adjustments(pixels: &mut [u32], effects: ThemeImageEffects) {
    let brightness = f32::from(effects.brightness) / 100.0;
    let contrast = f32::from(effects.contrast) / 100.0;
    let saturation = f32::from(effects.saturation) / 100.0;
    let grayscale = f32::from(effects.grayscale) / 255.0;
    let tint_alpha = ((effects.tint >> 24) & 0xFF) as f32 / 255.0;
    let tint_red = ((effects.tint >> 16) & 0xFF) as f32;
    let tint_green = ((effects.tint >> 8) & 0xFF) as f32;
    let tint_blue = (effects.tint & 0xFF) as f32;

    for pixel in pixels {
        let alpha = *pixel & 0xFF00_0000;
        if alpha == 0 {
            continue;
        }
        let mut red = ((*pixel >> 16) & 0xFF) as f32;
        let mut green = ((*pixel >> 8) & 0xFF) as f32;
        let mut blue = (*pixel & 0xFF) as f32;

        let luminance = red * 0.2126 + green * 0.7152 + blue * 0.0722;
        red = luminance + (red - luminance) * saturation;
        green = luminance + (green - luminance) * saturation;
        blue = luminance + (blue - luminance) * saturation;

        if grayscale > 0.0 {
            let gray = red * 0.2126 + green * 0.7152 + blue * 0.0722;
            red += (gray - red) * grayscale;
            green += (gray - green) * grayscale;
            blue += (gray - blue) * grayscale;
        }

        red = ((red - 128.0) * contrast + 128.0) * brightness;
        green = ((green - 128.0) * contrast + 128.0) * brightness;
        blue = ((blue - 128.0) * contrast + 128.0) * brightness;

        if tint_alpha > 0.0 {
            red += (tint_red - red) * tint_alpha;
            green += (tint_green - green) * tint_alpha;
            blue += (tint_blue - blue) * tint_alpha;
        }

        *pixel = alpha
            | ((red.round().clamp(0.0, 255.0) as u32) << 16)
            | ((green.round().clamp(0.0, 255.0) as u32) << 8)
            | blue.round().clamp(0.0, 255.0) as u32;
    }
}

fn rotate_argb(source: &[u32], width: usize, height: usize, degrees: i16) -> Vec<u32> {
    let radians = f32::from(degrees).to_radians();
    let sine = radians.sin();
    let cosine = radians.cos();
    let center_x = (width.saturating_sub(1)) as f32 / 2.0;
    let center_y = (height.saturating_sub(1)) as f32 / 2.0;
    let mut output = vec![0_u32; width.saturating_mul(height)];
    for y in 0..height {
        for x in 0..width {
            let destination_x = x as f32 - center_x;
            let destination_y = y as f32 - center_y;
            let source_x = destination_x * cosine + destination_y * sine + center_x;
            let source_y = -destination_x * sine + destination_y * cosine + center_y;
            let sample_x = source_x.round() as isize;
            let sample_y = source_y.round() as isize;
            if sample_x >= 0
                && sample_y >= 0
                && sample_x < width as isize
                && sample_y < height as isize
            {
                output[y * width + x] = source[sample_y as usize * width + sample_x as usize];
            }
        }
    }
    output
}

fn apply_rounded_corners(pixels: &mut [u32], width: usize, height: usize, radius: usize) {
    let radius = radius.min(width / 2).min(height / 2);
    if radius == 0 {
        return;
    }
    let doubled_radius = radius.saturating_mul(2);
    let radius_squared = (doubled_radius * doubled_radius) as i64;
    for y in 0..height {
        for x in 0..width {
            let (corner_x, corner_y) = if x < radius && y < radius {
                (radius - 1 - x, radius - 1 - y)
            } else if x >= width - radius && y < radius {
                (x - (width - radius), radius - 1 - y)
            } else if x < radius && y >= height - radius {
                (radius - 1 - x, y - (height - radius))
            } else if x >= width - radius && y >= height - radius {
                (x - (width - radius), y - (height - radius))
            } else {
                continue;
            };
            let corner_x = corner_x.saturating_mul(2).saturating_add(1);
            let corner_y = corner_y.saturating_mul(2).saturating_add(1);
            let distance = (corner_x * corner_x + corner_y * corner_y) as i64;
            if distance >= radius_squared {
                pixels[y * width + x] = 0;
            }
        }
    }
}

fn apply_gradient_overlay(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    start: u32,
    end: u32,
    direction: ThemeGradientDirection,
) {
    let extent = match direction {
        ThemeGradientDirection::Vertical => height,
        ThemeGradientDirection::Horizontal => width,
    }
    .saturating_sub(1)
    .max(1);
    for y in 0..height {
        for x in 0..width {
            let position = match direction {
                ThemeGradientDirection::Vertical => y,
                ThemeGradientDirection::Horizontal => x,
            };
            let overlay = interpolate_argb(start, end, position, extent);
            let index = y * width + x;
            let image_alpha = pixels[index] >> 24;
            let overlay_alpha = (overlay >> 24) * image_alpha / 255;
            let clipped_overlay = (overlay_alpha << 24) | (overlay & 0x00FF_FFFF);
            pixels[index] = source_over(clipped_overlay, pixels[index]);
        }
    }
}

fn interpolate_argb(start: u32, end: u32, position: usize, extent: usize) -> u32 {
    let interpolate = |shift: u32| {
        let start_channel = ((start >> shift) & 0xFF_u32) as i64;
        let end_channel = ((end >> shift) & 0xFF_u32) as i64;
        (start_channel + (end_channel - start_channel) * position as i64 / extent as i64)
            .clamp(0, 255) as u32
    };
    (interpolate(24) << 24) | (interpolate(16) << 16) | (interpolate(8) << 8) | interpolate(0)
}

fn box_blur_argb(source: &[u32], width: usize, height: usize, radius: usize) -> Vec<u32> {
    if radius == 0 {
        return source.to_vec();
    }
    let mut horizontal = vec![0_u32; source.len()];
    let mut prefixes = vec![[0_u64; 4]; width + 1];
    for y in 0..height {
        prefixes.fill([0; 4]);
        for x in 0..width {
            let pixel = source[y * width + x];
            let previous = prefixes[x];
            prefixes[x + 1] = [
                previous[0] + u64::from((pixel >> 24) & 0xFF),
                previous[1] + u64::from((pixel >> 16) & 0xFF),
                previous[2] + u64::from((pixel >> 8) & 0xFF),
                previous[3] + u64::from(pixel & 0xFF),
            ];
        }
        for x in 0..width {
            let start = x.saturating_sub(radius);
            let end = (x + radius + 1).min(width);
            horizontal[y * width + x] =
                average_channels(prefixes[start], prefixes[end], end - start);
        }
    }

    let mut output = vec![0_u32; source.len()];
    prefixes.resize(height + 1, [0; 4]);
    for x in 0..width {
        prefixes.fill([0; 4]);
        for y in 0..height {
            let pixel = horizontal[y * width + x];
            let previous = prefixes[y];
            prefixes[y + 1] = [
                previous[0] + u64::from((pixel >> 24) & 0xFF),
                previous[1] + u64::from((pixel >> 16) & 0xFF),
                previous[2] + u64::from((pixel >> 8) & 0xFF),
                previous[3] + u64::from(pixel & 0xFF),
            ];
        }
        for y in 0..height {
            let start = y.saturating_sub(radius);
            let end = (y + radius + 1).min(height);
            output[y * width + x] = average_channels(prefixes[start], prefixes[end], end - start);
        }
    }
    output
}

fn average_channels(start: [u64; 4], end: [u64; 4], count: usize) -> u32 {
    let count = count.max(1) as u64;
    let average = |channel| ((end[channel] - start[channel] + count / 2) / count) as u32;
    (average(0) << 24) | (average(1) << 16) | (average(2) << 8) | average(3)
}

fn box_blur_mask(source: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    if radius == 0 {
        return source.to_vec();
    }
    let mut horizontal = vec![0_u8; source.len()];
    let mut prefix = vec![0_u64; width + 1];
    for y in 0..height {
        prefix.fill(0);
        for x in 0..width {
            prefix[x + 1] = prefix[x] + u64::from(source[y * width + x]);
        }
        for x in 0..width {
            let start = x.saturating_sub(radius);
            let end = (x + radius + 1).min(width);
            horizontal[y * width + x] =
                ((prefix[end] - prefix[start]) / (end - start).max(1) as u64) as u8;
        }
    }

    let mut output = vec![0_u8; source.len()];
    prefix.resize(height + 1, 0);
    for x in 0..width {
        prefix.fill(0);
        for y in 0..height {
            prefix[y + 1] = prefix[y] + u64::from(horizontal[y * width + x]);
        }
        for y in 0..height {
            let start = y.saturating_sub(radius);
            let end = (y + radius + 1).min(height);
            output[y * width + x] =
                ((prefix[end] - prefix[start]) / (end - start).max(1) as u64) as u8;
        }
    }
    output
}

fn composite_colored_mask(
    destination: &mut [u32],
    width: usize,
    height: usize,
    mask: &[u8],
    offset_x: i16,
    offset_y: i16,
    color: u32,
) {
    let color_alpha = (color >> 24) & 0xFF;
    for source_y in 0..height {
        let target_y = source_y as i32 + i32::from(offset_y);
        if !(0..height as i32).contains(&target_y) {
            continue;
        }
        for source_x in 0..width {
            let target_x = source_x as i32 + i32::from(offset_x);
            if !(0..width as i32).contains(&target_x) {
                continue;
            }
            let alpha = u32::from(mask[source_y * width + source_x]) * color_alpha / 255;
            if alpha == 0 {
                continue;
            }
            let source = (alpha << 24) | (color & 0x00FF_FFFF);
            let index = target_y as usize * width + target_x as usize;
            destination[index] = source_over(source, destination[index]);
        }
    }
}

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
    let blend = |shift: u32| {
        let source_channel = (source >> shift) & 0xFF_u32;
        let destination_channel = (destination >> shift) & 0xFF_u32;
        let numerator = source_channel * source_alpha * 255
            + destination_channel * destination_alpha * inverse_source_alpha;
        (numerator + output_alpha_numerator / 2) / output_alpha_numerator
    };
    let output_alpha = (output_alpha_numerator + 127) / 255;
    (output_alpha << 24) | (blend(16) << 16) | (blend(8) << 8) | blend(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend_theme::{ThemeGlow, ThemeGradient, ThemeShadow};

    #[test]
    fn neutral_effects_preserve_pixels() {
        let source = [0xFFFF_0000, 0x8000_FF00, 0x0000_0000, 0xFF00_00FF];
        assert_eq!(
            apply_image_effects(&source, 2, 2, ThemeImageEffects::default()),
            source
        );
    }

    #[test]
    fn grayscale_and_tint_keep_original_alpha() {
        let effects = ThemeImageEffects {
            grayscale: 255,
            tint: 0x80FF_0000,
            ..ThemeImageEffects::default()
        };
        let output = apply_image_effects(&[0x8040_80C0], 1, 1, effects);
        assert_eq!(output[0] >> 24, 0x80);
        assert!((output[0] >> 16 & 0xFF) > (output[0] & 0xFF));
    }

    #[test]
    fn rounded_corners_remove_corner_alpha() {
        let effects = ThemeImageEffects {
            rounded_corners: 2,
            ..ThemeImageEffects::default()
        };
        let output = apply_image_effects(&[0xFFFF_FFFF; 16], 4, 4, effects);
        assert_eq!(output[0] >> 24, 0);
        assert_eq!(output[5] >> 24, 255);
    }

    #[test]
    fn shadow_and_glow_render_behind_source() {
        let mut source = [0_u32; 25];
        source[12] = 0xFFFF_FFFF;
        let effects = ThemeImageEffects {
            shadow: Some(ThemeShadow {
                color: 0xFF00_0000,
                x: 1,
                y: 1,
                blur: 0,
            }),
            glow: Some(ThemeGlow {
                color: 0x8000_FFFF,
                radius: 1,
            }),
            ..ThemeImageEffects::default()
        };
        let output = apply_image_effects(&source, 5, 5, effects);
        assert_eq!(output[12], 0xFFFF_FFFF);
        assert_ne!(output[18] >> 24, 0);
        assert_ne!(output[11] >> 24, 0);
    }

    #[test]
    fn gradient_interpolates_both_ends() {
        let effects = ThemeImageEffects {
            gradient: Some(ThemeGradient {
                start: 0x0000_0000,
                end: 0xFFFF_0000,
                direction: ThemeGradientDirection::Horizontal,
            }),
            ..ThemeImageEffects::default()
        };
        let output = apply_image_effects(&[0xFF00_00FF; 3], 3, 1, effects);
        assert_eq!(output[0], 0xFF00_00FF);
        assert_eq!(output[2], 0xFFFF_0000);
    }

    #[test]
    fn animation_combines_entry_and_pulse_transforms() {
        let animation = ThemeAnimation {
            fade_in_ms: 1_000,
            slide_in: ThemeSlideDirection::Left,
            slide_distance: 100,
            slide_duration_ms: 1_000,
            zoom_in_ms: 1_000,
            zoom_from: 50,
            pulse: 0,
            pulse_period_ms: 1_000,
        };
        let start = animate_visual(
            ThemeRect::new(100, 100, 200, 100),
            200,
            animation,
            Duration::ZERO,
        );
        assert_eq!(start.opacity, 0);
        assert!(start.rect.x < 100);
        assert_eq!(start.rect.width, 100);

        let end = animate_visual(
            ThemeRect::new(100, 100, 200, 100),
            200,
            animation,
            Duration::from_secs(1),
        );
        assert_eq!(end.opacity, 200);
        assert_eq!(end.rect, ThemeRect::new(100, 100, 200, 100));
        assert!(!animation_needs_redraw(animation, Duration::from_secs(1)));
    }
}

use std::path::Path;

const BMP_FILE_HEADER_SIZE: u32 = 14;
const BMP_INFO_HEADER_SIZE: u32 = 40;
const BMP_PIXEL_OFFSET: u32 = BMP_FILE_HEADER_SIZE + BMP_INFO_HEADER_SIZE;
const BYTES_PER_PIXEL: usize = 4;

pub fn save_framebuffer_bmp<P: AsRef<Path>>(
    path: P,
    framebuffer: &[u32],
    width: usize,
    height: usize,
) -> Result<(), String> {
    let bytes = encode_bmp(framebuffer, width, height)?;
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("No se pudo crear directorio {:?}: {error}", parent))?;
    }
    std::fs::write(&path, bytes)
        .map_err(|error| format!("No se pudo guardar captura {:?}: {error}", path.as_ref()))
}

fn encode_bmp(framebuffer: &[u32], width: usize, height: usize) -> Result<Vec<u8>, String> {
    let expected_len = width * height;
    if framebuffer.len() != expected_len {
        return Err(format!(
            "Framebuffer inválido: {} píxeles, esperado {expected_len}",
            framebuffer.len()
        ));
    }

    let pixel_bytes = expected_len
        .checked_mul(BYTES_PER_PIXEL)
        .ok_or_else(|| "Framebuffer demasiado grande para BMP".to_string())?;
    let file_size = (BMP_PIXEL_OFFSET as usize)
        .checked_add(pixel_bytes)
        .ok_or_else(|| "BMP demasiado grande".to_string())?;

    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend(b"BM");
    bmp.extend((file_size as u32).to_le_bytes());
    bmp.extend([0u8; 4]);
    bmp.extend(BMP_PIXEL_OFFSET.to_le_bytes());
    bmp.extend(BMP_INFO_HEADER_SIZE.to_le_bytes());
    bmp.extend((width as i32).to_le_bytes());
    bmp.extend((height as i32).to_le_bytes());
    bmp.extend(1u16.to_le_bytes());
    bmp.extend(32u16.to_le_bytes());
    bmp.extend(0u32.to_le_bytes());
    bmp.extend((pixel_bytes as u32).to_le_bytes());
    bmp.extend(2835u32.to_le_bytes());
    bmp.extend(2835u32.to_le_bytes());
    bmp.extend(0u32.to_le_bytes());
    bmp.extend(0u32.to_le_bytes());

    for row in (0..height).rev() {
        let start = row * width;
        let end = start + width;
        for pixel in &framebuffer[start..end] {
            let [b, g, r, a] = pixel.to_le_bytes();
            bmp.extend([b, g, r, a]);
        }
    }

    Ok(bmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_bmp_header_and_pixel_data_size() {
        let framebuffer = [0xFF112233, 0xFF445566];
        let bmp = encode_bmp(&framebuffer, 2, 1).unwrap();

        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(
            bmp.len(),
            BMP_PIXEL_OFFSET as usize + framebuffer.len() * BYTES_PER_PIXEL
        );
        assert_eq!(
            &bmp[BMP_PIXEL_OFFSET as usize..BMP_PIXEL_OFFSET as usize + 4],
            &[0x33, 0x22, 0x11, 0xFF]
        );
    }

    #[test]
    fn rejects_wrong_framebuffer_size() {
        assert!(encode_bmp(&[0], 2, 1).is_err());
    }
}

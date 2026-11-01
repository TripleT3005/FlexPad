// Constants

const ICON_FILE: &[u8] = include_bytes!("../../assets/icons/icon.ico");
const BITMAP_HEADER_SIZE: usize = 40;

// Icon

pub fn icon_pixels(size: u16) -> Vec<u8> {
    let bitmap =
        bitmap(size).unwrap_or_else(|| panic!("assets/icons/icon.ico has no {size} px bitmap"));
    let row = usize::from(size) * 4;
    bitmap[BITMAP_HEADER_SIZE..BITMAP_HEADER_SIZE + row * usize::from(size)]
        .chunks(row)
        .rev()
        .flat_map(|line| {
            line.chunks(4)
                .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], pixel[3]])
        })
        .collect()
}

fn bitmap(size: u16) -> Option<&'static [u8]> {
    let count = usize::from(u16::from_le_bytes([*ICON_FILE.get(4)?, *ICON_FILE.get(5)?]));
    let pixel_bytes = usize::from(size) * usize::from(size) * 4;
    (0..count).find_map(|index| {
        let entry = ICON_FILE.get(6 + index * 16..22 + index * 16)?;
        let dimension = if entry[0] == 0 {
            256
        } else {
            u16::from(entry[0])
        };
        let length = usize::try_from(u32::from_le_bytes(entry[8..12].try_into().ok()?)).ok()?;
        let offset = usize::try_from(u32::from_le_bytes(entry[12..16].try_into().ok()?)).ok()?;
        let image = ICON_FILE.get(offset..offset + length)?;
        (dimension == size
            && image.starts_with(&[40, 0, 0, 0])
            && image.len() >= BITMAP_HEADER_SIZE + pixel_bytes)
            .then_some(image)
    })
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(pixels: &[u8], size: u16, x: u16, y: u16) -> [u8; 4] {
        let index = (usize::from(y) * usize::from(size) + usize::from(x)) * 4;
        pixels[index..index + 4].try_into().unwrap()
    }

    // Icon

    #[test]
    fn has_rgba_pixels_for_every_bitmap_size() {
        for size in [16, 20, 24, 32, 40, 48, 64, 80, 128] {
            assert_eq!(
                icon_pixels(size).len(),
                usize::from(size) * usize::from(size) * 4
            );
        }
    }

    #[test]
    fn shows_the_logo_shapes() {
        let pixels = icon_pixels(64);

        assert_eq!(pixel(&pixels, 64, 17, 17), [29, 158, 117, 255]);
        assert_eq!(pixel(&pixels, 64, 17, 47), [93, 202, 165, 255]);
        assert_eq!(pixel(&pixels, 64, 47, 47), [29, 158, 117, 255]);
        assert_eq!(pixel(&pixels, 64, 40, 20), [239, 159, 39, 255]);
        assert_eq!(pixel(&pixels, 64, 47, 12), [255, 255, 255, 255]);
    }

    #[test]
    fn keeps_gaps_and_corners_transparent() {
        let pixels = icon_pixels(64);

        assert_eq!(pixel(&pixels, 64, 31, 47)[3], 0);
        assert_eq!(pixel(&pixels, 64, 4, 4)[3], 0);
        assert_eq!(pixel(&pixels, 64, 63, 63)[3], 0);
    }
}

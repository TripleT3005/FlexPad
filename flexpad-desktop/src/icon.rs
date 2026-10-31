// Constants

const VIEWBOX: f32 = 64.0;
const SAMPLES: u16 = 4;
const TILE_SIZE: f32 = 26.0;
const TILE_RADIUS: f32 = 7.0;
const TILES: [((f32, f32), [f32; 3]); 3] = [
    ((4.0, 4.0), [29.0, 158.0, 117.0]),
    ((4.0, 34.0), [93.0, 202.0, 165.0]),
    ((34.0, 34.0), [29.0, 158.0, 117.0]),
];
const KNOB_CENTER: (f32, f32) = (47.0, 17.0);
const KNOB_RADIUS: f32 = 13.0;
const KNOB_COLOR: [f32; 3] = [239.0, 159.0, 39.0];
const MARKER: [(f32, f32); 2] = [(47.0, 17.0), (47.0, 8.0)];
const MARKER_WIDTH: f32 = 3.5;
const MARKER_COLOR: [f32; 3] = [255.0, 255.0, 255.0];

// Icon

pub fn icon_pixels(size: u16) -> Vec<u8> {
    let scale = VIEWBOX / f32::from(size);
    let samples = f32::from(SAMPLES * SAMPLES);
    let mut pixels = Vec::with_capacity(usize::from(size) * usize::from(size) * 4);
    for y in 0..size {
        for x in 0..size {
            let mut color = [0.0_f32; 3];
            let mut coverage = 0.0_f32;
            for sample_y in 0..SAMPLES {
                for sample_x in 0..SAMPLES {
                    let point = (
                        (f32::from(x) + (f32::from(sample_x) + 0.5) / f32::from(SAMPLES)) * scale,
                        (f32::from(y) + (f32::from(sample_y) + 0.5) / f32::from(SAMPLES)) * scale,
                    );
                    if let Some(sample) = color_at(point) {
                        for (channel, value) in color.iter_mut().zip(sample) {
                            *channel += value;
                        }
                        coverage += 1.0;
                    }
                }
            }
            if coverage == 0.0 {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                pixels.extend(color.map(|channel| to_byte(channel / coverage)));
                pixels.push(to_byte(coverage / samples * 255.0));
            }
        }
    }
    pixels
}

fn color_at(point: (f32, f32)) -> Option<[f32; 3]> {
    if distance_squared(point, KNOB_CENTER) <= KNOB_RADIUS.powi(2) {
        return Some(if is_on_line(point, MARKER) {
            MARKER_COLOR
        } else {
            KNOB_COLOR
        });
    }
    TILES
        .iter()
        .find(|(origin, _)| is_in_rounded_square(point, *origin))
        .map(|(_, color)| *color)
}

fn is_in_rounded_square((x, y): (f32, f32), (left, top): (f32, f32)) -> bool {
    let (right, bottom) = (left + TILE_SIZE, top + TILE_SIZE);
    if x < left || x > right || y < top || y > bottom {
        return false;
    }
    let nearest_x = x.clamp(left + TILE_RADIUS, right - TILE_RADIUS);
    let nearest_y = y.clamp(top + TILE_RADIUS, bottom - TILE_RADIUS);
    distance_squared((x, y), (nearest_x, nearest_y)) <= TILE_RADIUS.powi(2)
}

fn is_on_line((x, y): (f32, f32), [(start_x, start_y), (end_x, end_y)]: [(f32, f32); 2]) -> bool {
    let (delta_x, delta_y) = (end_x - start_x, end_y - start_y);
    let length = delta_x.mul_add(delta_x, delta_y * delta_y);
    let position =
        ((x - start_x).mul_add(delta_x, (y - start_y) * delta_y) / length).clamp(0.0, 1.0);
    let nearest = (
        position.mul_add(delta_x, start_x),
        position.mul_add(delta_y, start_y),
    );
    distance_squared((x, y), nearest) <= (MARKER_WIDTH / 2.0).powi(2)
}

fn distance_squared((x, y): (f32, f32), (other_x, other_y): (f32, f32)) -> f32 {
    (x - other_x).powi(2) + (y - other_y).powi(2)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(pixels: &[u8], size: u16, x: u16, y: u16) -> [u8; 4] {
        let index = (usize::from(y) * usize::from(size) + usize::from(x)) * 4;
        pixels[index..index + 4].try_into().unwrap()
    }

    // Icon

    #[test]
    fn has_rgba_pixels_for_every_size() {
        for size in [16, 32, 48, 256] {
            assert_eq!(
                icon_pixels(size).len(),
                usize::from(size) * usize::from(size) * 4
            );
        }
    }

    #[test]
    fn draws_the_logo_shapes() {
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

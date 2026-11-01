// Constants

const END: u8 = 0xC0;
const ESCAPE: u8 = 0xDB;
const ESCAPED_END: u8 = 0xDC;
const ESCAPED_ESCAPE: u8 = 0xDD;

// Encoding

pub fn encode(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 2);
    frame.push(END);
    for &byte in payload {
        match byte {
            END => frame.extend_from_slice(&[ESCAPE, ESCAPED_END]),
            ESCAPE => frame.extend_from_slice(&[ESCAPE, ESCAPED_ESCAPE]),
            _ => frame.push(byte),
        }
    }
    frame.push(END);
    frame
}

// Decoding

#[derive(Debug, Default)]
pub struct Decoder {
    frame: Vec<u8>,
    in_frame: bool,
    escaped: bool,
}

impl Decoder {
    pub fn push(&mut self, byte: u8) -> Option<Vec<u8>> {
        if byte == END {
            self.escaped = false;
            if self.in_frame && !self.frame.is_empty() {
                return Some(std::mem::take(&mut self.frame));
            }
            self.in_frame = true;
            return None;
        }
        if !self.in_frame {
            return None;
        }
        if self.escaped {
            self.escaped = false;
            match byte {
                ESCAPED_END => self.frame.push(END),
                ESCAPED_ESCAPE => self.frame.push(ESCAPE),
                _ => {
                    self.frame.clear();
                    self.in_frame = false;
                }
            }
        } else if byte == ESCAPE {
            self.escaped = true;
        } else {
            self.frame.push(byte);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_all(bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut decoder = Decoder::default();
        bytes.iter().filter_map(|&byte| decoder.push(byte)).collect()
    }

    // Encoding

    #[test]
    fn frames_and_escapes_special_bytes() {
        assert_eq!(
            encode(&[0x01, 0xC0, 0x02, 0xDB, 0x03]),
            [0xC0, 0x01, 0xDB, 0xDC, 0x02, 0xDB, 0xDD, 0x03, 0xC0]
        );
        assert_eq!(encode(&[]), [0xC0, 0xC0]);
    }

    // Decoding

    #[test]
    fn round_trips_every_byte_value() {
        let payload: Vec<u8> = (0..=255).collect();

        assert_eq!(decode_all(&encode(&payload)), [payload]);
    }

    #[test]
    fn splits_back_to_back_frames() {
        let mut bytes = encode(&[1, 2]);
        bytes.extend(encode(&[3]));

        assert_eq!(decode_all(&bytes), [vec![1, 2], vec![3]]);
    }

    #[test]
    fn ignores_noise_outside_frames_and_empty_frames() {
        let mut bytes = b"ESP-ROM:esp32s3\r\n".to_vec();
        bytes.extend([0xC0, 0xC0, 0xC0]);
        bytes.extend(encode(&[7]));

        assert_eq!(decode_all(&bytes), [vec![7]]);
    }

    #[test]
    fn drops_frames_with_invalid_escapes() {
        let mut bytes = vec![0xC0, 0x01, 0xDB, 0x00, 0x02, 0xC0];
        bytes.extend(encode(&[9]));

        assert_eq!(decode_all(&bytes), [vec![9]]);
    }
}

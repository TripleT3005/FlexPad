// Constants

pub const SLOT_SIZE: u32 = 32 * 1024;
pub const SLOT_COUNT: u32 = 2;
pub const PARTITION_SIZE: u32 = SLOT_SIZE * SLOT_COUNT;
pub const HEADER_SIZE: usize = 16;
const MAGIC: [u8; 4] = *b"FPC1";
const WORD_SIZE: usize = 4;
const CHUNK_SIZE: usize = 256;

// Flash access

pub trait ConfigFlash {
    type Error;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;
    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError<E> {
    Flash(E),
    TooLarge,
}

impl<E> From<E> for StorageError<E> {
    fn from(error: E) -> Self {
        StorageError::Flash(error)
    }
}

// Slots

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredConfig {
    pub slot: u32,
    pub sequence: u32,
    pub length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Header {
    sequence: u32,
    length: u32,
    crc: u32,
}

impl Header {
    fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0; HEADER_SIZE];
        bytes[..4].copy_from_slice(&MAGIC);
        bytes[4..8].copy_from_slice(&self.sequence.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.length.to_le_bytes());
        bytes[12..].copy_from_slice(&self.crc.to_le_bytes());
        bytes
    }

    fn decode(bytes: &[u8; HEADER_SIZE]) -> Option<Self> {
        let word = |index: usize| {
            u32::from_le_bytes([
                bytes[index],
                bytes[index + 1],
                bytes[index + 2],
                bytes[index + 3],
            ])
        };
        (bytes[..4] == MAGIC).then(|| Self {
            sequence: word(4),
            length: word(8),
            crc: word(12),
        })
    }
}

pub const fn max_data_size() -> usize {
    SLOT_SIZE as usize - HEADER_SIZE
}

pub fn load<F: ConfigFlash>(
    flash: &mut F,
    buffer: &mut [u8],
) -> Result<Option<StoredConfig>, F::Error> {
    let mut best: Option<StoredConfig> = None;
    for slot in 0..SLOT_COUNT {
        let Some(found) = read_slot(flash, slot, buffer)? else {
            continue;
        };
        if best.is_none_or(|current| found.sequence > current.sequence) {
            best = Some(found);
        }
    }
    if let Some(found) = best {
        read_slot(flash, found.slot, buffer)?;
    }
    Ok(best)
}

fn read_slot<F: ConfigFlash>(
    flash: &mut F,
    slot: u32,
    buffer: &mut [u8],
) -> Result<Option<StoredConfig>, F::Error> {
    let base = slot * SLOT_SIZE;
    let mut header_bytes = [0; HEADER_SIZE];
    flash.read(base, &mut header_bytes)?;
    let Some(header) = Header::decode(&header_bytes) else {
        return Ok(None);
    };
    let length = header.length as usize;
    let padded = length.next_multiple_of(WORD_SIZE);
    if length > max_data_size() || padded > buffer.len() {
        return Ok(None);
    }
    flash.read(base + HEADER_SIZE as u32, &mut buffer[..padded])?;
    if crc32(&buffer[..length]) != header.crc {
        return Ok(None);
    }
    Ok(Some(StoredConfig {
        slot,
        sequence: header.sequence,
        length,
    }))
}

pub fn save<F: ConfigFlash>(
    flash: &mut F,
    current: Option<StoredConfig>,
    data: &[u8],
) -> Result<StoredConfig, StorageError<F::Error>> {
    if data.len() > max_data_size() {
        return Err(StorageError::TooLarge);
    }
    let (slot, sequence) = match current {
        Some(stored) => (
            (stored.slot + 1) % SLOT_COUNT,
            stored.sequence.wrapping_add(1),
        ),
        None => (0, 1),
    };
    let base = slot * SLOT_SIZE;
    flash.erase(base, base + SLOT_SIZE)?;

    let mut chunk = [0xFF; CHUNK_SIZE];
    let mut offset = 0;
    while offset < data.len() {
        let length = (data.len() - offset).min(CHUNK_SIZE);
        let padded = length.next_multiple_of(WORD_SIZE);
        chunk[..length].copy_from_slice(&data[offset..offset + length]);
        chunk[length..padded].fill(0xFF);
        flash.write(base + (HEADER_SIZE + offset) as u32, &chunk[..padded])?;
        offset += length;
    }

    let header = Header {
        sequence,
        length: data.len() as u32,
        crc: crc32(data),
    };
    flash.write(base, &header.encode())?;
    Ok(StoredConfig {
        slot,
        sequence,
        length: data.len(),
    })
}

// Checksum

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test flash

    struct NorFlash {
        bytes: Vec<u8>,
        fail_after_writes: Option<usize>,
    }

    impl NorFlash {
        fn new() -> Self {
            Self {
                bytes: vec![0xFF; PARTITION_SIZE as usize],
                fail_after_writes: None,
            }
        }
    }

    impl ConfigFlash for NorFlash {
        type Error = ();

        fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), ()> {
            assert_eq!(offset as usize % WORD_SIZE, 0);
            assert_eq!(bytes.len() % WORD_SIZE, 0);
            let start = offset as usize;
            bytes.copy_from_slice(&self.bytes[start..start + bytes.len()]);
            Ok(())
        }

        fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), ()> {
            assert_eq!(offset as usize % WORD_SIZE, 0);
            assert_eq!(bytes.len() % WORD_SIZE, 0);
            if let Some(remaining) = self.fail_after_writes.as_mut() {
                if *remaining == 0 {
                    return Err(());
                }
                *remaining -= 1;
            }
            let start = offset as usize;
            for (target, &byte) in self.bytes[start..start + bytes.len()].iter_mut().zip(bytes) {
                *target &= byte;
            }
            Ok(())
        }

        fn erase(&mut self, from: u32, to: u32) -> Result<(), ()> {
            assert_eq!(from % 4096, 0);
            assert_eq!(to % 4096, 0);
            self.bytes[from as usize..to as usize].fill(0xFF);
            Ok(())
        }
    }

    fn load_text(flash: &mut NorFlash) -> Option<String> {
        let mut buffer = vec![0; max_data_size()];
        let stored = load(flash, &mut buffer).unwrap()?;
        Some(String::from_utf8(buffer[..stored.length].to_vec()).unwrap())
    }

    // Checksum

    #[test]
    fn crc32_matches_reference() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    // Slots

    #[test]
    fn empty_flash_has_no_config() {
        assert_eq!(load_text(&mut NorFlash::new()), None);
    }

    #[test]
    fn saved_config_can_be_loaded() {
        let mut flash = NorFlash::new();

        let stored = save(&mut flash, None, br#"{"tiles":[]}"#).unwrap();

        assert_eq!(
            stored,
            StoredConfig {
                slot: 0,
                sequence: 1,
                length: 12
            }
        );
        assert_eq!(load_text(&mut flash).as_deref(), Some(r#"{"tiles":[]}"#));
    }

    #[test]
    fn saves_alternate_between_slots_and_newest_wins() {
        let mut flash = NorFlash::new();
        let first = save(&mut flash, None, b"first").unwrap();
        let second = save(&mut flash, Some(first), b"second, longer than a chunk? no").unwrap();
        let third = save(&mut flash, Some(second), b"third").unwrap();

        assert_eq!((first.slot, second.slot, third.slot), (0, 1, 0));
        assert_eq!(third.sequence, 3);
        assert_eq!(load_text(&mut flash).as_deref(), Some("third"));
    }

    #[test]
    fn interrupted_save_keeps_previous_config() {
        let mut flash = NorFlash::new();
        let first = save(&mut flash, None, b"working config").unwrap();
        flash.fail_after_writes = Some(1);

        let data = vec![b'x'; 3 * CHUNK_SIZE];
        assert!(save(&mut flash, Some(first), &data).is_err());

        assert_eq!(load_text(&mut flash).as_deref(), Some("working config"));
    }

    #[test]
    fn corrupted_slot_is_ignored() {
        let mut flash = NorFlash::new();
        let first = save(&mut flash, None, b"good").unwrap();
        let second = save(&mut flash, Some(first), b"newer").unwrap();
        let data_offset = (second.slot * SLOT_SIZE) as usize + HEADER_SIZE;
        flash.bytes[data_offset] ^= 0x01;

        assert_eq!(load_text(&mut flash).as_deref(), Some("good"));
    }

    #[test]
    fn large_configs_span_many_chunks() {
        let mut flash = NorFlash::new();
        let data: String = (0..max_data_size())
            .map(|index| char::from(b'a' + (index % 26) as u8))
            .collect();

        save(&mut flash, None, data.as_bytes()).unwrap();

        assert_eq!(load_text(&mut flash), Some(data));
    }

    #[test]
    fn rejects_data_larger_than_a_slot() {
        let mut flash = NorFlash::new();
        let data = vec![0; max_data_size() + 1];

        assert_eq!(save(&mut flash, None, &data), Err(StorageError::TooLarge));
    }
}

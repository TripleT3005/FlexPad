// Constants

const IMAGE_MAGIC: u8 = 0xE9;
const FLASH_SIZE_BYTE: usize = 3;
const MEGABYTE: u32 = 0x10_0000;
const MAX_FLASH_SIZE_CODE: u8 = 7;
const PARTITION_TABLE_OFFSET: usize = 0x8000;
const PARTITION_TABLE_END: usize = 0x9000;
const PARTITION_ENTRY_LEN: usize = 32;
const PARTITION_MAGIC: [u8; 2] = [0xAA, 0x50];
const PARTITION_LABEL: std::ops::Range<usize> = 12..28;
const TYPE_APP: u8 = 0x00;
const TYPE_DATA: u8 = 0x01;
const SUBTYPE_FACTORY: u8 = 0x00;
const SUBTYPE_OTA_0: u8 = 0x10;
pub const CONFIG_PARTITION: &str = "flexpad";

// Partitions

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub label: String,
    pub kind: u8,
    pub subtype: u8,
    pub offset: u32,
    pub size: u32,
}

impl Partition {
    pub fn end(&self) -> u32 {
        self.offset.saturating_add(self.size)
    }
}

pub fn parse_partition_table(table: &[u8]) -> Vec<Partition> {
    table
        .as_chunks::<PARTITION_ENTRY_LEN>()
        .0
        .iter()
        .take_while(|entry| entry[..2] == PARTITION_MAGIC)
        .map(|entry| Partition {
            label: String::from_utf8_lossy(&entry[PARTITION_LABEL])
                .trim_end_matches('\0')
                .to_string(),
            kind: entry[2],
            subtype: entry[3],
            offset: u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]),
            size: u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]),
        })
        .collect()
}

// Image

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment<'a> {
    pub offset: u32,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareImage<'a> {
    data: &'a [u8],
    pub flash_size: u32,
    pub app_offset: u32,
    pub config_offset: u32,
}

impl<'a> FirmwareImage<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, String> {
        if data.first() != Some(&IMAGE_MAGIC) {
            return Err("the firmware image has no bootloader at offset 0".to_string());
        }
        let size_code = data.get(FLASH_SIZE_BYTE).map_or(u8::MAX, |byte| byte >> 4);
        if size_code > MAX_FLASH_SIZE_CODE {
            return Err("the firmware image has an unknown flash size".to_string());
        }
        let flash_size = MEGABYTE << size_code;
        let table = data
            .get(PARTITION_TABLE_OFFSET..PARTITION_TABLE_END)
            .ok_or("the firmware image has no partition table")?;
        let partitions = parse_partition_table(table);
        let app = partitions
            .iter()
            .find(|p| p.kind == TYPE_APP && matches!(p.subtype, SUBTYPE_FACTORY | SUBTYPE_OTA_0))
            .ok_or("the partition table has no app partition")?;
        let config = partitions
            .iter()
            .find(|p| p.kind == TYPE_DATA && p.label == CONFIG_PARTITION)
            .ok_or("the partition table has no FlexPad configuration partition")?;
        let length = data.len() as u64;
        if (app.offset as usize) < PARTITION_TABLE_END || u64::from(app.offset) >= length {
            return Err("the firmware image does not contain the app".to_string());
        }
        if data[app.offset as usize] != IMAGE_MAGIC {
            return Err("the app partition holds no app image".to_string());
        }
        if length > u64::from(app.end()) || length > u64::from(config.offset) {
            return Err("the firmware image would overwrite the configuration".to_string());
        }
        if config.end() > flash_size {
            return Err("the configuration partition lies outside the flash".to_string());
        }
        Ok(Self {
            data,
            flash_size,
            app_offset: app.offset,
            config_offset: config.offset,
        })
    }

    pub fn segments(&self) -> [Segment<'a>; 3] {
        let app_offset = self.app_offset as usize;
        [
            Segment {
                offset: 0,
                data: &self.data[..PARTITION_TABLE_OFFSET],
            },
            Segment {
                offset: PARTITION_TABLE_OFFSET as u32,
                data: &self.data[PARTITION_TABLE_OFFSET..PARTITION_TABLE_END],
            },
            Segment {
                offset: self.app_offset,
                data: &self.data[app_offset..],
            },
        ]
    }

    pub fn written_len(&self) -> usize {
        self.segments()
            .iter()
            .map(|segment| segment.data.len())
            .sum()
    }
}

// Embedded image

const EMBEDDED_IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/firmware.bin"));

pub fn embedded_image() -> Option<FirmwareImage<'static>> {
    FirmwareImage::parse(EMBEDDED_IMAGE).ok()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    const PARTITIONS_CSV: &str = include_str!("../../../flexpad-firmware/partitions.csv");

    fn csv_partitions() -> Vec<Partition> {
        PARTITIONS_CSV
            .lines()
            .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
            .map(|line| {
                let fields: Vec<&str> = line.split(',').map(str::trim).collect();
                let number = |field: &str| {
                    u32::from_str_radix(field.trim_start_matches("0x").trim_start_matches("0X"), 16)
                        .unwrap()
                };
                let (kind, subtype) = match (fields[1], fields[2]) {
                    ("app", "factory") => (TYPE_APP, SUBTYPE_FACTORY),
                    ("app", _) => (TYPE_APP, SUBTYPE_OTA_0),
                    ("data", "nvs") => (TYPE_DATA, 0x02),
                    ("data", "phy") => (TYPE_DATA, 0x01),
                    _ => (TYPE_DATA, 0x06),
                };
                Partition {
                    label: fields[0].to_string(),
                    kind,
                    subtype,
                    offset: number(fields[3]),
                    size: number(fields[4]),
                }
            })
            .collect()
    }

    pub fn partition_table(partitions: &[Partition]) -> Vec<u8> {
        let mut table = Vec::new();
        for partition in partitions {
            let mut label = [0u8; 16];
            label[..partition.label.len()].copy_from_slice(partition.label.as_bytes());
            table.extend(PARTITION_MAGIC);
            table.extend([partition.kind, partition.subtype]);
            table.extend(partition.offset.to_le_bytes());
            table.extend(partition.size.to_le_bytes());
            table.extend(label);
            table.extend([0; 4]);
        }
        table.resize(PARTITION_TABLE_END - PARTITION_TABLE_OFFSET, 0xFF);
        table
    }

    pub fn test_image(app_len: usize) -> Vec<u8> {
        let mut image = vec![0xFF; PARTITION_TABLE_OFFSET];
        image[..4].copy_from_slice(&[IMAGE_MAGIC, 0x03, 0x02, 0x3F]);
        image.extend(partition_table(&csv_partitions()));
        image.resize(0x1_0000, 0xFF);
        image.extend((0..app_len).map(|i| (i % 251) as u8));
        image[0x1_0000] = IMAGE_MAGIC;
        image
    }

    // Partition table

    #[test]
    fn reads_the_firmware_partition_table() {
        let partitions = parse_partition_table(&partition_table(&csv_partitions()));

        assert_eq!(partitions, csv_partitions());
        assert_eq!(partitions.len(), 4);
    }

    #[test]
    fn configuration_lies_behind_the_app() {
        let partitions = csv_partitions();
        let app = partitions.iter().find(|p| p.kind == TYPE_APP).unwrap();
        let config = partitions
            .iter()
            .find(|p| p.label == CONFIG_PARTITION)
            .unwrap();

        assert!(app.end() <= config.offset);
    }

    // Image layout

    #[test]
    fn accepts_a_merged_image() {
        let data = test_image(0x2345);
        let image = FirmwareImage::parse(&data).unwrap();

        assert_eq!(image.flash_size, 0x80_0000);
        assert_eq!(image.app_offset, 0x1_0000);
        assert_eq!(image.config_offset, 0x41_0000);
        let segments = image.segments();
        assert_eq!(
            segments.map(|segment| (segment.offset, segment.data.len())),
            [(0, 0x8000), (0x8000, 0x1000), (0x1_0000, 0x2345)]
        );
        assert_eq!(segments[2].data, &data[0x1_0000..]);
        assert_eq!(image.written_len(), 0x9000 + 0x2345);
    }

    #[test]
    fn skips_the_gap_before_the_app() {
        let data = test_image(16);
        let image = FirmwareImage::parse(&data).unwrap();

        assert!(
            image
                .segments()
                .iter()
                .all(|s| s.offset >= 0x1_0000 || s.offset as usize + s.data.len() <= 0x9000)
        );
    }

    #[test]
    fn rejects_images_that_reach_the_configuration() {
        let data = test_image(0x40_0000 + 1);

        assert_eq!(
            FirmwareImage::parse(&data),
            Err("the firmware image would overwrite the configuration".to_string())
        );
        assert!(FirmwareImage::parse(&test_image(0x40_0000)).is_ok());
    }

    #[test]
    fn rejects_broken_images() {
        assert!(FirmwareImage::parse(&[]).is_err());
        assert!(FirmwareImage::parse(&test_image(16)[..0x8800]).is_err());

        let mut no_app = test_image(16);
        no_app[0x1_0000] = 0;
        assert!(FirmwareImage::parse(&no_app).is_err());

        let mut no_config = test_image(16);
        no_config[0x8000 + 3 * 32..0x8000 + 4 * 32].fill(0xFF);
        assert_eq!(
            FirmwareImage::parse(&no_config),
            Err("the partition table has no FlexPad configuration partition".to_string())
        );

        let mut small_flash = test_image(16);
        small_flash[3] = 0x2F;
        assert!(FirmwareImage::parse(&small_flash).is_err());
    }

    #[test]
    fn embedded_image_is_valid_when_present() {
        if !EMBEDDED_IMAGE.is_empty() {
            let image = embedded_image().expect("the embedded firmware image is invalid");
            assert!(image.written_len() > 0);
        }
    }
}

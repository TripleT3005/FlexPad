// Constants

pub const FLASH_BLOCK_SIZE: usize = 0x400;
const DIRECTION_REQUEST: u8 = 0x00;
const DIRECTION_RESPONSE: u8 = 0x01;
const REQUEST_HEADER_LEN: usize = 8;
const RESPONSE_HEADER_LEN: usize = 8;
const ROM_STATUS_LEN: usize = 4;
const CHECKSUM_SEED: u8 = 0xEF;
const SYNC_PAYLOAD_HEADER: [u8; 4] = [0x07, 0x07, 0x12, 0x20];
const SYNC_PAYLOAD_FILL: u8 = 0x55;
const SYNC_PAYLOAD_LEN: usize = 36;
const FLASH_BLOCK_FILL: u8 = 0xFF;
const FLASH_BLOCK_64K: u32 = 0x1_0000;
const FLASH_SECTOR_SIZE: u32 = 0x1000;
const FLASH_PAGE_SIZE: u32 = 0x100;
const FLASH_STATUS_MASK: u32 = 0xFFFF;
const ALL_BITS: u32 = 0xFFFF_FFFF;

// Opcodes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    FlashBegin = 0x02,
    FlashData = 0x03,
    FlashEnd = 0x04,
    Sync = 0x08,
    WriteReg = 0x09,
    ReadReg = 0x0A,
    SpiSetParams = 0x0B,
    SpiAttach = 0x0D,
}

// Requests

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub opcode: Opcode,
    pub data: Vec<u8>,
    pub checksum: u32,
}

impl Request {
    fn new(opcode: Opcode, words: &[u32]) -> Self {
        Self {
            opcode,
            data: words.iter().flat_map(|word| word.to_le_bytes()).collect(),
            checksum: 0,
        }
    }

    pub fn sync() -> Self {
        let mut data = SYNC_PAYLOAD_HEADER.to_vec();
        data.resize(SYNC_PAYLOAD_LEN, SYNC_PAYLOAD_FILL);
        Self {
            opcode: Opcode::Sync,
            data,
            checksum: 0,
        }
    }

    pub fn spi_attach() -> Self {
        Self::new(Opcode::SpiAttach, &[0, 0])
    }

    pub fn spi_set_params(flash_size: u32) -> Self {
        Self::new(
            Opcode::SpiSetParams,
            &[
                0,
                flash_size,
                FLASH_BLOCK_64K,
                FLASH_SECTOR_SIZE,
                FLASH_PAGE_SIZE,
                FLASH_STATUS_MASK,
            ],
        )
    }

    pub fn flash_begin(size: u32, offset: u32) -> Self {
        let blocks = size.div_ceil(FLASH_BLOCK_SIZE as u32);
        Self::new(
            Opcode::FlashBegin,
            &[size, blocks, FLASH_BLOCK_SIZE as u32, offset, 0],
        )
    }

    pub fn flash_data(sequence: u32, block: &[u8]) -> Self {
        let mut payload = block.to_vec();
        payload.resize(FLASH_BLOCK_SIZE.max(block.len()), FLASH_BLOCK_FILL);
        let mut request = Self::new(Opcode::FlashData, &[payload.len() as u32, sequence, 0, 0]);
        request.checksum = checksum(&payload);
        request.data.extend(payload);
        request
    }

    pub fn flash_end(reboot: bool) -> Self {
        Self::new(Opcode::FlashEnd, &[u32::from(!reboot)])
    }

    pub fn read_reg(address: u32) -> Self {
        Self::new(Opcode::ReadReg, &[address])
    }

    pub fn write_reg(address: u32, value: u32) -> Self {
        Self::write_reg_masked(address, value, ALL_BITS)
    }

    pub fn write_reg_masked(address: u32, value: u32, mask: u32) -> Self {
        Self::new(Opcode::WriteReg, &[address, value, mask, 0])
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(REQUEST_HEADER_LEN + self.data.len());
        bytes.push(DIRECTION_REQUEST);
        bytes.push(self.opcode as u8);
        bytes.extend((self.data.len() as u16).to_le_bytes());
        bytes.extend(self.checksum.to_le_bytes());
        bytes.extend(&self.data);
        bytes
    }
}

pub fn checksum(data: &[u8]) -> u32 {
    u32::from(data.iter().fold(CHECKSUM_SEED, |sum, byte| sum ^ byte))
}

// Responses

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Response {
    pub opcode: u8,
    pub value: u32,
    pub status: u8,
    pub error: u8,
}

impl Response {
    pub fn parse(frame: &[u8]) -> Option<Self> {
        let (header, rest) = frame.split_at_checked(RESPONSE_HEADER_LEN)?;
        if header[0] != DIRECTION_RESPONSE {
            return None;
        }
        let size = usize::from(u16::from_le_bytes([header[2], header[3]]));
        let data = rest.get(..size)?;
        let status = &data[data.len().saturating_sub(ROM_STATUS_LEN)..];
        Some(Self {
            opcode: header[1],
            value: u32::from_le_bytes([header[4], header[5], header[6], header[7]]),
            status: status.first().copied().unwrap_or(0),
            error: status.get(1).copied().unwrap_or(0),
        })
    }

    pub fn failure(&self) -> Option<String> {
        if self.status == 0 {
            return None;
        }
        let reason = match self.error {
            0x05 => "invalid message",
            0x06 => "command failed",
            0x07 => "checksum mismatch",
            0x08 => "flash write error",
            0x09 => "flash read error",
            0x0A => "flash read length error",
            0x0B => "deflate error",
            _ => "unknown error",
        };
        Some(format!("{reason} (code 0x{:02X})", self.error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(bytes: &[u8]) -> Vec<u32> {
        bytes
            .chunks(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .collect()
    }

    // Checksum

    #[test]
    fn checksum_xors_from_the_seed() {
        assert_eq!(checksum(&[]), 0xEF);
        assert_eq!(checksum(&[0xEF]), 0x00);
        assert_eq!(checksum(&[0x01, 0x02, 0x04]), 0xEF ^ 0x07);
    }

    // Requests

    #[test]
    fn encodes_the_request_header() {
        let bytes = Request::read_reg(0x6000_4038).to_bytes();

        assert_eq!(bytes[..8], [0x00, 0x0A, 0x04, 0x00, 0, 0, 0, 0]);
        assert_eq!(bytes[8..], [0x38, 0x40, 0x00, 0x60]);
    }

    #[test]
    fn sync_carries_the_rom_pattern() {
        let request = Request::sync();

        assert_eq!(request.opcode, Opcode::Sync);
        assert_eq!(request.data.len(), 36);
        assert_eq!(request.data[..4], [0x07, 0x07, 0x12, 0x20]);
        assert!(request.data[4..].iter().all(|&byte| byte == 0x55));
    }

    #[test]
    fn flash_begin_counts_blocks_and_disables_encryption() {
        let request = Request::flash_begin(0x0C01, 0x8000);

        assert_eq!(request.opcode, Opcode::FlashBegin);
        assert_eq!(words(&request.data), [0x0C01, 4, 0x400, 0x8000, 0]);
    }

    #[test]
    fn flash_data_pads_the_block_and_sets_the_checksum() {
        let request = Request::flash_data(3, &[0x12, 0x34]);
        let bytes = request.to_bytes();

        assert_eq!(words(&request.data[..16]), [0x400, 3, 0, 0]);
        assert_eq!(request.data.len(), 16 + 0x400);
        assert!(request.data[18..].iter().all(|&byte| byte == 0xFF));
        assert_eq!(request.checksum, checksum(&request.data[16..]));
        assert_eq!(bytes[2..4], (16u16 + 0x400).to_le_bytes());
        assert_eq!(bytes[4..8], request.checksum.to_le_bytes());
    }

    #[test]
    fn other_requests_encode_their_parameters() {
        assert_eq!(words(&Request::spi_attach().data), [0, 0]);
        assert_eq!(
            words(&Request::spi_set_params(0x80_0000).data),
            [0, 0x80_0000, 0x1_0000, 0x1000, 0x100, 0xFFFF]
        );
        assert_eq!(words(&Request::flash_end(false).data), [1]);
        assert_eq!(words(&Request::flash_end(true).data), [0]);
        assert_eq!(
            words(&Request::write_reg_masked(0x6000_812C, 0, 1).data),
            [0x6000_812C, 0, 1, 0]
        );
        assert_eq!(
            words(&Request::write_reg(0x6000_80B0, 0x50D8_3AA1).data),
            [0x6000_80B0, 0x50D8_3AA1, 0xFFFF_FFFF, 0]
        );
    }

    // Responses

    #[test]
    fn parses_rom_responses_with_four_status_bytes() {
        let frame = [0x01, 0x0A, 0x04, 0x00, 0x78, 0x56, 0x34, 0x12, 0, 0, 0, 0];

        assert_eq!(
            Response::parse(&frame),
            Some(Response {
                opcode: 0x0A,
                value: 0x1234_5678,
                status: 0,
                error: 0,
            })
        );
    }

    #[test]
    fn reports_failures() {
        let frame = [0x01, 0x03, 0x04, 0x00, 0, 0, 0, 0, 0x01, 0x07, 0, 0];
        let response = Response::parse(&frame).unwrap();

        assert_eq!(
            response.failure().as_deref(),
            Some("checksum mismatch (code 0x07)")
        );
    }

    #[test]
    fn rejects_requests_and_truncated_frames() {
        assert_eq!(Response::parse(&Request::sync().to_bytes()), None);
        assert_eq!(Response::parse(&[0x01, 0x08, 0x04, 0x00, 0, 0, 0]), None);
        assert_eq!(
            Response::parse(&[0x01, 0x08, 0x04, 0x00, 0, 0, 0, 0, 0]),
            None
        );
    }
}

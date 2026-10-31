#![no_std]
#![no_main]
#![deny(clippy::mem_forget, clippy::large_stack_frames)]

use core::fmt::Write;

use embassy_executor::Spawner;
use embassy_futures::join::join3;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Timer;
use embassy_usb::{
    Builder,
    class::{
        cdc_acm::{self, CdcAcmClass},
        hid::{self, HidBootProtocol, HidSubclass, HidWriter},
    },
    driver::EndpointError,
};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};
use esp_backtrace as _;
use esp_bootloader_esp_idf::partitions::{self, PARTITION_TABLE_MAX_LEN};
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    efuse,
    peripherals::LPWR,
    system,
    timer::timg::TimerGroup,
    usb::otg::{
        Usb,
        embassy_usb_device::{Config, Driver},
    },
};
use esp_storage::{FlashStorage, FlashStorageError};
use flexpad_firmware::{
    config::{self, Bindings, ConfigError, MAX_CONFIG_SIZE},
    hid::{KeyboardState, MAX_REPORT_SIZE, REPORT_DESCRIPTOR, consumer_report},
    macros::{self, Action},
    protocol::{
        self, CONFIG_DATA_PREFIX, Command, DeviceInfo, EMPTY_CONFIG, LineReader, MAX_LINE_LENGTH,
        PRODUCT_NAME, TextBuffer,
    },
    storage::{self, ConfigFlash, PARTITION_SIZE, StoredConfig},
};
use static_cell::{ConstStaticCell, StaticCell};

esp_bootloader_esp_idf::esp_app_desc!();

// Constants

const USB_VENDOR_ID: u16 = 0x303A;
const USB_PRODUCT_ID: u16 = 0x1001;
const MAX_PACKET_SIZE: usize = 64;
const HID_PACKET_SIZE: u16 = 16;
const HID_POLL_MS: u8 = 5;
const MEDIA_PRESS_MS: u64 = 30;
const MAX_MACRO_LENGTH: usize = 512;
const MACRO_QUEUE_SIZE: usize = 4;
const CONFIG_PARTITION_LABEL: &str = "flexpad";
const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

type MacroText = TextBuffer<MAX_MACRO_LENGTH>;
type Response = TextBuffer<256>;

// Static state

static SERIAL_NUMBER: StaticCell<TextBuffer<32>> = StaticCell::new();
static EP_OUT_BUFFER: ConstStaticCell<[u8; 1024]> = ConstStaticCell::new([0; 1024]);
static CONFIG_DESCRIPTOR: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0; 256]);
static BOS_DESCRIPTOR: ConstStaticCell<[u8; 256]> = ConstStaticCell::new([0; 256]);
static CONTROL_BUFFER: ConstStaticCell<[u8; 64]> = ConstStaticCell::new([0; 64]);
static CDC_STATE: StaticCell<cdc_acm::State> = StaticCell::new();
static HID_STATE: StaticCell<hid::State> = StaticCell::new();
static LINE_READER: ConstStaticCell<LineReader<MAX_LINE_LENGTH>> =
    ConstStaticCell::new(LineReader::new());
static RESPONSE: ConstStaticCell<Response> = ConstStaticCell::new(TextBuffer::new());
static CONFIG_JSON: ConstStaticCell<[u8; MAX_CONFIG_SIZE]> =
    ConstStaticCell::new([0; MAX_CONFIG_SIZE]);
static BINDINGS: ConstStaticCell<Bindings> = ConstStaticCell::new(Bindings::new());
static SCRATCH_BINDINGS: ConstStaticCell<Bindings> = ConstStaticCell::new(Bindings::new());
static MACRO_QUEUE: Channel<CriticalSectionRawMutex, MacroText, MACRO_QUEUE_SIZE> = Channel::new();

// Entry point

#[allow(
    clippy::large_stack_frames,
    reason = "the USB device lives for the whole program"
)]
#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let serial_number = SERIAL_NUMBER.init(TextBuffer::new());
    let mac: [u8; 6] = efuse::base_mac_address()
        .as_bytes()
        .try_into()
        .unwrap_or_default();
    let _ = protocol::write_serial_number(&mac, serial_number);
    let serial_number: &'static TextBuffer<32> = serial_number;
    let serial_number = serial_number.as_str();

    let mut config_state = ConfigState::load(FlashStorage::new(peripherals.FLASH));

    let usb = Usb::new_fs(peripherals.USB_FS, peripherals.GPIO20, peripherals.GPIO19);
    let driver = Driver::new(usb, EP_OUT_BUFFER.take(), Config::default());

    let mut usb_config = embassy_usb::Config::new(USB_VENDOR_ID, USB_PRODUCT_ID);
    usb_config.manufacturer = Some(PRODUCT_NAME);
    usb_config.product = Some(PRODUCT_NAME);
    usb_config.serial_number = Some(serial_number);
    usb_config.max_power = 100;
    usb_config.max_packet_size_0 = MAX_PACKET_SIZE as u8;

    let mut builder = Builder::new(
        driver,
        usb_config,
        CONFIG_DESCRIPTOR.take(),
        BOS_DESCRIPTOR.take(),
        &mut [],
        CONTROL_BUFFER.take(),
    );
    let mut serial = CdcAcmClass::new(
        &mut builder,
        CDC_STATE.init(cdc_acm::State::new()),
        MAX_PACKET_SIZE as u16,
    );
    let mut keyboard = HidWriter::<_, MAX_REPORT_SIZE>::new(
        &mut builder,
        HID_STATE.init(hid::State::new()),
        hid::Config {
            report_descriptor: REPORT_DESCRIPTOR,
            request_handler: None,
            poll_ms: HID_POLL_MS,
            max_packet_size: HID_PACKET_SIZE,
            hid_subclass: HidSubclass::No,
            hid_boot_protocol: HidBootProtocol::None,
        },
    );
    let mut usb_device = builder.build();

    let mut session = Session {
        info: DeviceInfo {
            serial_number,
            firmware_version: FIRMWARE_VERSION,
        },
        line_reader: LINE_READER.take(),
        response: RESPONSE.take(),
        config: &mut config_state,
    };

    let serial_future = async {
        loop {
            serial.wait_connection().await;
            let _ = session.handle_connection(&mut serial).await;
        }
    };

    match join3(usb_device.run(), serial_future, run_macros(&mut keyboard))
        .await
        .0 {}
}

// Serial protocol

struct Disconnected;

impl From<EndpointError> for Disconnected {
    fn from(_: EndpointError) -> Self {
        Disconnected
    }
}

enum Reply {
    Response,
    Config,
    Reboot,
}

struct Session<'a> {
    info: DeviceInfo<'a>,
    line_reader: &'a mut LineReader<MAX_LINE_LENGTH>,
    response: &'a mut Response,
    config: &'a mut ConfigState,
}

impl Session<'_> {
    async fn handle_connection<'d>(
        &mut self,
        serial: &mut CdcAcmClass<'d, Driver<'d>>,
    ) -> Result<(), Disconnected> {
        let mut packet = [0u8; MAX_PACKET_SIZE];
        loop {
            let length = serial.read_packet(&mut packet).await?;
            for &byte in &packet[..length] {
                let Some(reply) = self.handle_byte(byte) else {
                    continue;
                };
                match reply {
                    Reply::Response => write_all(serial, self.response.as_bytes()).await?,
                    Reply::Config => {
                        write_all(serial, CONFIG_DATA_PREFIX.as_bytes()).await?;
                        write_all(serial, self.config.json()).await?;
                        write_all(serial, b"\n").await?;
                    }
                    Reply::Reboot => {
                        write_all(serial, self.response.as_bytes()).await?;
                        reboot_to_bootloader();
                    }
                }
            }
        }
    }

    fn handle_byte(&mut self, byte: u8) -> Option<Reply> {
        self.response.clear();
        let line = match self.line_reader.push(byte)? {
            Ok(line) => line,
            Err(_) => {
                let _ = protocol::write_line_too_long(self.response);
                return Some(Reply::Response);
            }
        };
        let command = protocol::parse_command(line);
        match command {
            Command::GetConfig => return Some(Reply::Config),
            Command::SetConfig(payload) => {
                let _ = match self.config.save(payload) {
                    Ok(()) => protocol::write_config_saved(self.response),
                    Err(SaveError::Invalid(error)) => {
                        protocol::write_config_error(error, self.response)
                    }
                    Err(SaveError::Storage) => protocol::write_storage_error(self.response),
                };
                return Some(Reply::Response);
            }
            _ => {}
        }
        let _ = protocol::write_response(command, &self.info, self.response);
        if let Command::RunMacro(payload) = command
            && !queue_macro(payload)
        {
            self.response.clear();
            let _ = protocol::write_macro_busy(self.response);
        }
        Some(if command == Command::RebootBootloader {
            Reply::Reboot
        } else {
            Reply::Response
        })
    }
}

async fn write_all<'d>(
    serial: &mut CdcAcmClass<'d, Driver<'d>>,
    data: &[u8],
) -> Result<(), Disconnected> {
    for chunk in data.chunks(MAX_PACKET_SIZE) {
        serial.write_packet(chunk).await?;
    }
    if data.len().is_multiple_of(MAX_PACKET_SIZE) {
        serial.write_packet(&[]).await?;
    }
    Ok(())
}

// Configuration

enum SaveError {
    Invalid(ConfigError),
    Storage,
}

struct PartitionFlash {
    storage: FlashStorage<'static>,
    offset: u32,
}

impl ConfigFlash for PartitionFlash {
    type Error = FlashStorageError;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        ReadNorFlash::read(&mut self.storage, self.offset + offset, bytes)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        NorFlash::write(&mut self.storage, self.offset + offset, bytes)
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        NorFlash::erase(&mut self.storage, self.offset + from, self.offset + to)
    }
}

struct ConfigState {
    flash: Option<PartitionFlash>,
    stored: Option<StoredConfig>,
    json: &'static mut [u8; MAX_CONFIG_SIZE],
    json_length: usize,
    bindings: &'static mut Bindings,
    scratch: &'static mut Bindings,
}

impl ConfigState {
    fn load(mut storage: FlashStorage<'static>) -> Self {
        let mut state = Self {
            flash: None,
            stored: None,
            json: CONFIG_JSON.take(),
            json_length: 0,
            bindings: BINDINGS.take(),
            scratch: SCRATCH_BINDINGS.take(),
        };
        let Some(offset) = find_config_partition(&mut storage) else {
            return state;
        };
        let mut flash = PartitionFlash { storage, offset };
        if let Ok(Some(stored)) = storage::load(&mut flash, state.json.as_mut_slice())
            && config::parse_config(&state.json[..stored.length], state.bindings).is_ok()
        {
            state.stored = Some(stored);
            state.json_length = stored.length;
        }
        state.flash = Some(flash);
        state
    }

    fn json(&self) -> &[u8] {
        if self.json_length == 0 {
            EMPTY_CONFIG.as_bytes()
        } else {
            &self.json[..self.json_length]
        }
    }

    fn save(&mut self, payload: &[u8]) -> Result<(), SaveError> {
        config::parse_config(payload, self.scratch).map_err(SaveError::Invalid)?;
        let flash = self.flash.as_mut().ok_or(SaveError::Storage)?;
        let stored = storage::save(flash, self.stored, payload).map_err(|_| SaveError::Storage)?;
        self.json[..payload.len()].copy_from_slice(payload);
        self.json_length = payload.len();
        self.stored = Some(stored);
        core::mem::swap(&mut self.bindings, &mut self.scratch);
        Ok(())
    }
}

fn find_config_partition(storage: &mut FlashStorage<'static>) -> Option<u32> {
    let mut table_buffer = [0u8; PARTITION_TABLE_MAX_LEN];
    let table = partitions::read_partition_table(storage, &mut table_buffer).ok()?;
    let entry = table
        .iter()
        .find(|entry| entry.label_as_str() == CONFIG_PARTITION_LABEL)?;
    (entry.len() >= PARTITION_SIZE).then(|| entry.offset())
}

// Macros

fn queue_macro(payload: &[u8]) -> bool {
    let Ok(text) = protocol::macro_text(payload) else {
        return true;
    };
    let mut macro_text = MacroText::new();
    if macro_text.write_str(text).is_err() {
        return false;
    }
    MACRO_QUEUE.try_send(macro_text).is_ok()
}

async fn run_macros<'d>(keyboard: &mut HidWriter<'d, Driver<'d>, MAX_REPORT_SIZE>) -> ! {
    let mut state = KeyboardState::new();
    loop {
        let macro_text = MACRO_QUEUE.receive().await;
        for action in macros::actions(macro_text.as_str()).flatten() {
            match action {
                Action::Press(key) => {
                    state.press(key);
                    let _ = keyboard.write(&state.report()).await;
                }
                Action::Release(key) => {
                    state.release(key);
                    let _ = keyboard.write(&state.report()).await;
                }
                Action::Delay(ms) => Timer::after_millis(u64::from(ms)).await,
                Action::Media(usage) => {
                    let _ = keyboard.write(&consumer_report(usage)).await;
                    Timer::after_millis(MEDIA_PRESS_MS).await;
                    let _ = keyboard.write(&consumer_report(0)).await;
                }
            }
        }
        if !state.is_empty() {
            state.release_all();
            let _ = keyboard.write(&state.report()).await;
        }
    }
}

// System

fn reboot_to_bootloader() -> ! {
    Delay::new().delay_millis(100);
    LPWR::regs()
        .option1()
        .modify(|_, w| w.force_download_boot().set_bit());
    system::software_reset()
}

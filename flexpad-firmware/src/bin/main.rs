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
use esp_backtrace as _;
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
use flexpad_firmware::{
    hid::{KeyboardState, MAX_REPORT_SIZE, REPORT_DESCRIPTOR, consumer_report},
    macros::{self, Action},
    protocol::{self, Command, DeviceInfo, LineReader, MAX_LINE_LENGTH, PRODUCT_NAME, TextBuffer},
};
use static_cell::StaticCell;

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
const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

type MacroText = TextBuffer<MAX_MACRO_LENGTH>;

// Static state

static SERIAL_NUMBER: StaticCell<TextBuffer<32>> = StaticCell::new();
static EP_OUT_BUFFER: StaticCell<[u8; 1024]> = StaticCell::new();
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUFFER: StaticCell<[u8; 64]> = StaticCell::new();
static CDC_STATE: StaticCell<cdc_acm::State> = StaticCell::new();
static HID_STATE: StaticCell<hid::State> = StaticCell::new();
static LINE_READER: StaticCell<LineReader<MAX_LINE_LENGTH>> = StaticCell::new();
static RESPONSE: StaticCell<TextBuffer<256>> = StaticCell::new();
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

    let usb = Usb::new_fs(peripherals.USB_FS, peripherals.GPIO20, peripherals.GPIO19);
    let driver = Driver::new(usb, EP_OUT_BUFFER.init([0; 1024]), Config::default());

    let mut usb_config = embassy_usb::Config::new(USB_VENDOR_ID, USB_PRODUCT_ID);
    usb_config.manufacturer = Some(PRODUCT_NAME);
    usb_config.product = Some(PRODUCT_NAME);
    usb_config.serial_number = Some(serial_number);
    usb_config.max_power = 100;
    usb_config.max_packet_size_0 = MAX_PACKET_SIZE as u8;

    let mut builder = Builder::new(
        driver,
        usb_config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [],
        CONTROL_BUFFER.init([0; 64]),
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

    let info = DeviceInfo {
        serial_number,
        firmware_version: FIRMWARE_VERSION,
    };
    let line_reader = LINE_READER.init(LineReader::new());
    let response = RESPONSE.init(TextBuffer::new());

    let serial_future = async {
        loop {
            serial.wait_connection().await;
            let _ = handle_connection(&mut serial, &info, &mut *line_reader, &mut *response).await;
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

async fn handle_connection<'d>(
    serial: &mut CdcAcmClass<'d, Driver<'d>>,
    info: &DeviceInfo<'_>,
    line_reader: &mut LineReader<MAX_LINE_LENGTH>,
    response: &mut TextBuffer<256>,
) -> Result<(), Disconnected> {
    let mut packet = [0u8; MAX_PACKET_SIZE];
    loop {
        let length = serial.read_packet(&mut packet).await?;
        for &byte in &packet[..length] {
            response.clear();
            let command = match line_reader.push(byte) {
                Some(Ok(line)) => {
                    let command = protocol::parse_command(line);
                    let _ = protocol::write_response(command, info, response);
                    if let Command::RunMacro(payload) = command
                        && !queue_macro(payload)
                    {
                        response.clear();
                        let _ = protocol::write_macro_busy(response);
                    }
                    Some(command == Command::RebootBootloader)
                }
                Some(Err(_)) => {
                    let _ = protocol::write_line_too_long(response);
                    Some(false)
                }
                None => None,
            };
            let Some(reboot) = command else {
                continue;
            };
            write_all(serial, response.as_bytes()).await?;
            if reboot {
                reboot_to_bootloader();
            }
        }
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

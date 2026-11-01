use crate::app::{DeviceTask, FlexPadApp, Message};
use crate::flasher::{self, FlashStage};
use crate::keys::KeyboardLayout;
use crate::update::{self, APP_VERSION};
use iced::widget::{
    Space, Text, button, checkbox, column, container, pick_list, progress_bar, row, text,
    text_input,
};
use iced::{Alignment, Element, Length};

// Constants

const LABEL_WIDTH: f32 = 60.0;
const GROUP_SPACING: f32 = 24.0;

// Header

pub fn view_header(app: &FlexPadApp) -> Element<'_, Message> {
    let mut header = column![device_bar(app), profile_bar(app)].spacing(8);
    if let Some(notice) = firmware_notice(app) {
        header = header.push(notice);
    }
    if let Some(progress) = flash_progress(app) {
        header = header.push(progress);
    }
    header.into()
}

fn device_bar(app: &FlexPadApp) -> Element<'_, Message> {
    let is_idle = !app.is_device_busy();
    let device_picker = pick_list(
        app.available_devices.as_slice(),
        app.selected_device.clone(),
        Message::DeviceSelected,
    )
    .placeholder("Select device...")
    .width(260);
    let is_connected = app.connected_device.is_some();
    let can_use_device = is_idle && is_connected;
    let scan_btn =
        button("Scan").on_press_maybe((is_idle && !is_connected).then_some(Message::ScanDevices));
    let conn_btn = if is_connected {
        button("Disconnect").on_press_maybe(is_idle.then_some(Message::DisconnectDevice))
    } else {
        button("Connect").on_press_maybe(
            (is_idle && app.selected_device.is_some()).then_some(Message::ConnectDevice),
        )
    };
    let load_btn =
        button("Load Config").on_press_maybe(can_use_device.then_some(Message::LoadConfig));
    let save_btn =
        button("Set Config").on_press_maybe(can_use_device.then_some(Message::SetConfig));

    row![
        label("Device"),
        device_picker,
        scan_btn,
        conn_btn,
        Space::new().width(GROUP_SPACING),
        load_btn,
        save_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn profile_bar(app: &FlexPadApp) -> Element<'_, Message> {
    let profile_picker = pick_list(
        app.saved_profiles.as_slice(),
        app.selected_profile.clone(),
        Message::ProfileSelected,
    )
    .placeholder("Select profile...")
    .width(260);
    let name_input = text_input("Profile name", &app.current_profile_name)
        .on_input(Message::ProfileNameChanged)
        .width(200)
        .padding(5);
    let layout_picker = pick_list(
        KeyboardLayout::ALL,
        Some(app.keyboard_layout),
        Message::KeyboardLayoutSelected,
    )
    .width(150);
    let new_btn = button("New Profile").on_press(Message::NewProfile);
    let save_btn = button("Save Profile").on_press(Message::SaveCurrentProfile);

    row![
        label("Profile"),
        profile_picker,
        name_input,
        text("Layout").size(13),
        layout_picker,
        Space::new().width(GROUP_SPACING),
        new_btn,
        save_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn label(content: &str) -> Element<'_, Message> {
    text(content).size(13).width(LABEL_WIDTH).into()
}

// Firmware

fn firmware_notice(app: &FlexPadApp) -> Option<Element<'_, Message>> {
    let is_idle = !app.is_device_busy();
    let (message, action) = if let Some(firmware) = outdated_firmware(app) {
        (
            format!("Firmware {firmware} is older than the app ({APP_VERSION})."),
            "Update firmware",
        )
    } else if app.connected_device.is_none() && is_idle && app.download_port.is_some() {
        (
            "Board in bootloader mode found.".to_string(),
            "Install firmware",
        )
    } else {
        return None;
    };
    let mut content = row![text(message).size(13)]
        .spacing(12)
        .align_y(Alignment::Center);
    content = if has_firmware_image() {
        content.push(
            button(text(action).size(13))
                .padding([2, 10])
                .on_press_maybe(is_idle.then_some(Message::UpdateFirmware)),
        )
    } else {
        content.push(text("This build of the app contains no firmware.").size(13))
    };
    Some(banner(content.into()))
}

fn flash_progress(app: &FlexPadApp) -> Option<Element<'_, Message>> {
    if app.device_task != Some(DeviceTask::Flashing) {
        return None;
    }
    let (label, fraction) = match app.flash_stage {
        None | Some(FlashStage::Rebooting) => {
            ("Restarting the FlexPad into its bootloader...", 0.0)
        }
        Some(FlashStage::WaitingForBootloader) => ("Waiting for the bootloader...", 0.0),
        Some(FlashStage::Connecting) => ("Connecting to the bootloader...", 0.0),
        Some(FlashStage::Writing { written, total }) => (
            "Writing the firmware...",
            written as f32 / total.max(1) as f32,
        ),
        Some(FlashStage::Restarting) => ("Restarting the FlexPad...", 1.0),
    };
    Some(banner(
        column![
            text("⚠ Do not unplug the FlexPad or close the app until the update is finished.")
                .size(13),
            row![
                progress_bar(0.0..=1.0, fraction).length(300).girth(12),
                text(format!("{label} {:.0} %", fraction * 100.0)).size(12),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(4)
        .into(),
    ))
}

fn banner(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content)
        .padding([6, 12])
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn has_firmware_image() -> bool {
    flasher::embedded_image().is_some()
}

fn outdated_firmware(app: &FlexPadApp) -> Option<&str> {
    let firmware = app.connected_device.as_ref()?.firmware_version.as_deref()?;
    update::is_newer(APP_VERSION, firmware).then_some(firmware)
}

// Status bar

pub fn view_status_bar(app: &FlexPadApp) -> Element<'_, Message> {
    let (dot, label): (Text<'_>, &str) = if app.connected_device.is_some() {
        (text("●").style(text::success), "Connected")
    } else {
        (text("●").style(text::danger), "Disconnected")
    };
    let mut bar = row![
        row![dot.size(16), text(label).size(13)]
            .spacing(6)
            .align_y(Alignment::Center),
        text(&app.status_message).size(13),
        Space::new().width(Length::Fill),
    ]
    .spacing(15)
    .align_y(Alignment::Center);
    if let Some(release) = &app.available_update {
        bar = bar.push(
            button(text(format!("Version {} available", release.version)).size(13))
                .padding(0)
                .style(button::text)
                .on_press(Message::OpenReleasePage),
        );
    }
    bar.push(
        checkbox(app.check_for_updates)
            .label("Check for updates")
            .size(14)
            .text_size(13)
            .on_toggle(Message::UpdateCheckToggled),
    )
    .push(text(format!("v{APP_VERSION}")).size(13))
    .into()
}

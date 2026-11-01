use crate::app::{DeviceTask, FlexPadApp, Message};
use crate::flasher::{self, FlashStage};
use crate::keys::KeyboardLayout;
use crate::update::{self, APP_VERSION};
use iced::widget::{
    button, checkbox, column, container, pick_list, progress_bar, row, text, text_input,
};
use iced::{Element, Length};

pub fn view_header(app: &FlexPadApp) -> Element<'_, Message> {
    let status_message = app.status_message.as_str();
    let available_devices = &app.available_devices;
    let selected_device = app.selected_device.as_ref();
    let saved_profiles = &app.saved_profiles;
    let selected_profile = app.selected_profile.as_ref();
    let current_profile_name = app.current_profile_name.as_str();
    let keyboard_layout = app.keyboard_layout;

    let title = text("FlexPad Configurator").size(24);
    let is_connected = app.connected_device.is_some();
    let is_idle = !app.is_device_busy();
    let status_indicator = if is_connected {
        text("🟢 Connected").size(14)
    } else {
        text("🔴 Disconnected").size(14)
    };
    let status_row = row![status_indicator, text(status_message)].spacing(15);

    let update_check = checkbox(app.check_for_updates)
        .label("Check for updates")
        .size(14)
        .text_size(12)
        .on_toggle(Message::UpdateCheckToggled);
    let mut notices = row![update_check]
        .spacing(15)
        .align_y(iced::Alignment::Center);
    if let Some(release) = &app.available_update {
        notices = notices.push(
            button(text(format!("Version {} available", release.version)).size(12))
                .padding(0)
                .style(button::text)
                .on_press(Message::OpenReleasePage),
        );
    }
    if let Some(firmware) = outdated_firmware(app) {
        notices = notices.push(
            text(format!(
                "Firmware {firmware} is older than the app ({APP_VERSION})."
            ))
            .size(12),
        );
        notices = notices.push(firmware_button("Update firmware", is_idle));
    } else if !is_connected && is_idle && app.download_port.is_some() {
        notices = notices.push(text("Board in bootloader mode found.").size(12));
        notices = notices.push(firmware_button("Install firmware", is_idle));
    }
    if !has_firmware_image() && (outdated_firmware(app).is_some() || app.download_port.is_some()) {
        notices = notices.push(text("This build of the app contains no firmware.").size(12));
    }

    let scan_btn = button("Scan Devices").on_press_maybe(is_idle.then_some(Message::ScanDevices));
    let device_picker = pick_list(
        available_devices.to_vec(),
        selected_device.cloned(),
        Message::DeviceSelected,
    )
    .placeholder("Select device...");
    let conn_btn = if is_connected {
        button("Disconnect").on_press_maybe(is_idle.then_some(Message::DisconnectDevice))
    } else {
        button("Connect").on_press_maybe(is_idle.then_some(Message::ConnectDevice))
    };

    let load_btn = button("Load Config").on_press_maybe(is_idle.then_some(Message::LoadConfig));
    let save_btn = button("Set Config").on_press_maybe(is_idle.then_some(Message::SetConfig));

    let top_action_row = row![scan_btn, device_picker, conn_btn, load_btn, save_btn,].spacing(10);

    let profile_picker = pick_list(
        saved_profiles.to_vec(),
        selected_profile.cloned(),
        Message::ProfileSelected,
    )
    .placeholder("Select profile...")
    .width(220);

    let name_input = text_input("Profile name", current_profile_name)
        .on_input(Message::ProfileNameChanged)
        .width(220)
        .padding(5);

    let layout_picker = pick_list(
        KeyboardLayout::ALL,
        Some(keyboard_layout),
        Message::KeyboardLayoutSelected,
    )
    .width(220);

    let new_profile_btn = button("New Profile")
        .padding(8)
        .on_press(Message::NewProfile);
    let save_profile_btn = button("Save Profile")
        .padding(8)
        .on_press(Message::SaveCurrentProfile);

    let profile_controls_col = column![
        profile_picker,
        name_input,
        row![text("Keyboard layout:").size(12), layout_picker]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        row![new_profile_btn, save_profile_btn].spacing(10),
    ]
    .spacing(8);

    let mut main_col = column![title, status_row, notices].spacing(10);
    if let Some(progress) = flash_progress(app) {
        main_col = main_col.push(progress);
    }
    main_col = main_col.push(top_action_row);

    row![
        main_col.width(Length::Fill),
        container(profile_controls_col).align_x(iced::alignment::Horizontal::Right)
    ]
    .width(Length::Fill)
    .align_y(iced::Alignment::Center)
    .into()
}
fn has_firmware_image() -> bool {
    flasher::embedded_image().is_some()
}

fn firmware_button(label: &str, is_idle: bool) -> Element<'_, Message> {
    button(text(label).size(12))
        .padding([2, 8])
        .on_press_maybe((is_idle && has_firmware_image()).then_some(Message::UpdateFirmware))
        .into()
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
    Some(
        column![
            text("⚠ Do not unplug the FlexPad or close the app until the update is finished.")
                .size(13),
            row![
                progress_bar(0.0..=1.0, fraction).length(300).girth(12),
                text(format!("{label} {:.0} %", fraction * 100.0)).size(12),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(4)
        .into(),
    )
}

fn outdated_firmware(app: &FlexPadApp) -> Option<&str> {
    let firmware = app.connected_device.as_ref()?.firmware_version.as_deref()?;
    update::is_newer(APP_VERSION, firmware).then_some(firmware)
}

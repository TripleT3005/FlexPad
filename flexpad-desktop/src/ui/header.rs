use crate::app::{FlexPadApp, Message};
use crate::keys::KeyboardLayout;
use crate::update::{self, APP_VERSION};
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input};
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

    row![
        column![title, status_row, notices, top_action_row]
            .spacing(10)
            .width(Length::Fill),
        container(profile_controls_col).align_x(iced::alignment::Horizontal::Right)
    ]
    .width(Length::Fill)
    .align_y(iced::Alignment::Center)
    .into()
}
fn outdated_firmware(app: &FlexPadApp) -> Option<&str> {
    let firmware = app.connected_device.as_ref()?.firmware_version.as_deref()?;
    update::is_newer(APP_VERSION, firmware).then_some(firmware)
}

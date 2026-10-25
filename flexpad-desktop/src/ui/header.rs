use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};
use crate::app::{Message, ProfileItem};
use crate::device::FlexPadDeviceInfo;

pub fn view_header<'a>(
    status_message: &'a str,
    available_devices: &'a [FlexPadDeviceInfo],
    selected_device: Option<&'a FlexPadDeviceInfo>,
    connected_device: Option<&'a FlexPadDeviceInfo>,
    saved_profiles: &'a [ProfileItem],
    selected_profile: Option<&'a ProfileItem>,
    current_profile_name: &'a str,
) -> Element<'a, Message> {
    let title = text("FlexPad Configurator").size(24);
    let is_connected = connected_device.is_some();
    let status_indicator = if is_connected { text("🟢 Connected").size(14) } else { text("🔴 Disconnected").size(14) };
    let status_row = row![status_indicator, text(status_message)].spacing(15);

    let scan_btn = button("Scan Devices").on_press(Message::ScanDevices);
    let device_picker = pick_list(available_devices.to_vec(), selected_device.cloned(), Message::DeviceSelected).placeholder("Select device...");
    let conn_btn = if is_connected { button("Disconnect").on_press(Message::DisconnectDevice) } else { button("Connect").on_press(Message::ConnectDevice) };

    let load_btn = button("Load Config").on_press(Message::LoadConfig);
    let save_btn = button("Set Config").on_press(Message::SetConfig);

    let top_action_row = row![
        scan_btn,
        device_picker,
        conn_btn,
        load_btn,
        save_btn,
    ].spacing(10);

    let profile_picker = pick_list(
        saved_profiles.to_vec(),
        selected_profile.cloned(),
        Message::ProfileSelected,
    ).placeholder("Select profile...").width(220);

    let name_input = text_input("Profile name", current_profile_name)
        .on_input(Message::ProfileNameChanged)
        .width(220)
        .padding(5);

    let new_profile_btn = button("New Profile").padding(8).on_press(Message::NewProfile);
    let save_profile_btn = button("Save Profile").padding(8).on_press(Message::SaveCurrentProfile);

    let profile_controls_col = column![
        profile_picker,
        name_input,
        row![new_profile_btn, save_profile_btn].spacing(10),
    ].spacing(8);

    row![
        column![title, status_row, top_action_row]
            .spacing(10)
            .width(Length::Fill),
        container(profile_controls_col)
            .align_x(iced::alignment::Horizontal::Right)
    ]
    .width(Length::Fill)
    .align_y(iced::Alignment::Center)
    .into()
}
use crate::app::{DeviceTask, FlexPadApp, Message};
use crate::device::PROTOCOL_VERSION;
use crate::flasher::{self, FIRMWARE_VERSION, FlashStage};
use crate::i18n::{format_text, text as tr};
use crate::keys::KeyboardLayout;
use crate::texts;
use crate::ui::choice::Choice;
use crate::ui::label_button;
use crate::update::{self, APP_VERSION};
use iced::widget::{
    Space, Text, button, checkbox, column, container, pick_list, progress_bar, row, text,
    text_input,
};
use iced::{Alignment, Element, Length};

// Constants

const LABEL_WIDTH: f32 = 80.0;
const LANGUAGE_PICKER_WIDTH: f32 = 190.0;
const PICKER_WIDTH: f32 = 220.0;
const NAME_WIDTH: f32 = 170.0;
const GROUP_SPACING: f32 = 24.0;
const PROFILE_MENU_WIDTH: f32 = 200.0;

// Profile actions

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAction {
    New,
    Save,
    Import,
    Export,
    OpenFolder,
}

impl ProfileAction {
    pub const ALL: [Self; 5] = [
        Self::New,
        Self::Save,
        Self::Import,
        Self::Export,
        Self::OpenFolder,
    ];

    pub fn message(self) -> Message {
        match self {
            Self::New => Message::NewProfile,
            Self::Save => Message::SaveCurrentProfile,
            Self::Import => Message::ImportProfile,
            Self::Export => Message::ExportProfile,
            Self::OpenFolder => Message::OpenProfilesFolder,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::New => texts::PROFILE_NEW,
            Self::Save => texts::PROFILE_SAVE,
            Self::Import => texts::PROFILE_IMPORT,
            Self::Export => texts::PROFILE_EXPORT,
            Self::OpenFolder => texts::PROFILE_OPEN_FOLDER,
        }
    }
}

impl std::fmt::Display for ProfileAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&tr(self.label()))
    }
}

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
    .placeholder(tr(texts::DEVICE_SELECT))
    .width(PICKER_WIDTH);
    let is_connected = app.connected_device.is_some();
    let can_use_device = is_idle && is_connected;
    let scan_btn = label_button(tr(texts::DEVICE_SCAN))
        .on_press_maybe((is_idle && !is_connected).then_some(Message::ScanDevices));
    let conn_btn = if is_connected {
        label_button(tr(texts::DEVICE_DISCONNECT))
            .on_press_maybe(is_idle.then_some(Message::DisconnectDevice))
    } else {
        label_button(tr(texts::DEVICE_CONNECT)).on_press_maybe(
            (is_idle && app.selected_device.is_some()).then_some(Message::ConnectDevice),
        )
    };
    let load_btn = label_button(tr(texts::DEVICE_LOAD_CONFIG))
        .on_press_maybe(can_use_device.then_some(Message::LoadConfig));
    let save_btn = label_button(tr(texts::DEVICE_SET_CONFIG))
        .on_press_maybe(can_use_device.then_some(Message::SetConfig));

    row![
        label(tr(texts::DEVICE_LABEL)),
        device_picker,
        scan_btn,
        conn_btn,
        Space::new().width(GROUP_SPACING),
        load_btn,
        save_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap()
    .vertical_spacing(8)
    .into()
}

fn profile_bar(app: &FlexPadApp) -> Element<'_, Message> {
    let profile_picker = pick_list(
        app.saved_profiles.as_slice(),
        app.selected_profile.clone(),
        Message::ProfileSelected,
    )
    .placeholder(tr(texts::PROFILE_SELECT))
    .width(PICKER_WIDTH);
    let name_input = text_input(&tr(texts::PROFILE_NAME), &app.current_profile_name)
        .on_input(Message::ProfileNameChanged)
        .width(NAME_WIDTH)
        .padding(5);
    let layout_picker = pick_list(
        KeyboardLayout::by_name(),
        Some(app.keyboard_layout),
        Message::KeyboardLayoutSelected,
    );
    let profile_menu = pick_list(ProfileAction::ALL, None::<ProfileAction>, |action| {
        action.message()
    })
    .placeholder(tr(texts::PROFILE_LABEL))
    .width(PROFILE_MENU_WIDTH);

    row![
        label(tr(texts::PROFILE_LABEL)),
        profile_picker,
        name_input,
        text(tr(texts::PROFILE_LAYOUT)).size(13),
        layout_picker,
        Space::new().width(GROUP_SPACING),
        profile_menu,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap()
    .vertical_spacing(8)
    .into()
}

fn label<'a>(content: String) -> Element<'a, Message> {
    text(content).size(13).width(LABEL_WIDTH).into()
}

// Firmware

fn firmware_notice(app: &FlexPadApp) -> Option<Element<'_, Message>> {
    let is_idle = !app.is_device_busy();
    if needs_newer_app(app) {
        return Some(banner(
            text(tr(texts::FIRMWARE_NEWER_PROTOCOL)).size(13).into(),
        ));
    }
    let (message, action) = if let Some(firmware) = outdated_firmware(app) {
        (
            format_text(
                texts::FIRMWARE_OUTDATED,
                &[("firmware", firmware), ("version", FIRMWARE_VERSION)],
            ),
            tr(texts::FIRMWARE_UPDATE),
        )
    } else if app.connected_device.is_none() && is_idle && app.download_port.is_some() {
        (
            tr(texts::FIRMWARE_BOOTLOADER_FOUND),
            tr(texts::FIRMWARE_INSTALL),
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
        content.push(text(tr(texts::FIRMWARE_NOT_INCLUDED)).size(13))
    };
    Some(banner(content.into()))
}

fn flash_progress(app: &FlexPadApp) -> Option<Element<'_, Message>> {
    if app.device_task != Some(DeviceTask::Flashing) {
        return None;
    }
    let (label, fraction) = match app.flash_stage {
        None | Some(FlashStage::Rebooting) => (texts::FLASH_REBOOTING, 0.0),
        Some(FlashStage::WaitingForBootloader) => (texts::FLASH_WAITING, 0.0),
        Some(FlashStage::Connecting) => (texts::FLASH_CONNECTING, 0.0),
        Some(FlashStage::Writing { written, total }) => {
            (texts::FLASH_WRITING, written as f32 / total.max(1) as f32)
        }
        Some(FlashStage::Restarting) => (texts::FLASH_RESTARTING, 1.0),
    };
    Some(banner(
        column![
            text(format!("⚠ {}", tr(texts::FLASH_WARNING))).size(13),
            row![
                progress_bar(0.0..=1.0, fraction).length(300).girth(12),
                text(format_text(
                    texts::FLASH_PROGRESS,
                    &[
                        ("stage", &tr(label)),
                        ("percent", &format!("{:.0}", fraction * 100.0)),
                    ],
                ))
                .size(12),
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
    update::is_newer(FIRMWARE_VERSION, firmware).then_some(firmware)
}

fn needs_newer_app(app: &FlexPadApp) -> bool {
    app.connected_device
        .as_ref()
        .is_some_and(|device| device.protocol_version > PROTOCOL_VERSION)
}

// Status bar

pub fn view_status_bar(app: &FlexPadApp) -> Element<'_, Message> {
    let (dot, label): (Text<'_>, &str) = if app.connected_device.is_some() {
        (text("●").style(text::success), texts::STATUS_CONNECTED)
    } else {
        (text("●").style(text::danger), texts::STATUS_DISCONNECTED)
    };
    let mut bar = row![
        row![dot.size(16), text(tr(label)).size(13)]
            .spacing(6)
            .align_y(Alignment::Center),
        text(&app.status_message).size(13),
        Space::new().width(Length::Fill),
    ]
    .spacing(15)
    .align_y(Alignment::Center);
    bar = bar.push(language_picker(app));
    if let Some(release) = &app.available_update {
        bar = bar.push(
            button(
                text(format_text(
                    texts::STATUS_UPDATE_AVAILABLE,
                    &[("version", &release.version)],
                ))
                .size(13),
            )
            .padding(0)
            .style(button::text)
            .on_press(Message::OpenReleasePage),
        );
    }
    bar.push(
        checkbox(app.check_for_updates)
            .label(tr(texts::STATUS_CHECK_UPDATES))
            .size(14)
            .text_size(13)
            .on_toggle(Message::UpdateCheckToggled),
    )
    .push(text(format!("v{APP_VERSION}")).size(13))
    .into()
}

fn language_picker(app: &FlexPadApp) -> Element<'_, Message> {
    let mut choices = vec![Choice::new(None, tr(texts::LANGUAGE_AUTOMATIC))];
    choices.extend(
        app.languages
            .iter()
            .map(|language| Choice::new(Some(language.code.clone()), language.name.clone())),
    );
    let selected = choices
        .iter()
        .find(|choice| choice.value == app.language)
        .cloned()
        .or_else(|| choices.first().cloned());
    row![
        text(tr(texts::LANGUAGE_LABEL)).size(13),
        pick_list(choices, selected, |choice| Message::LanguageSelected(
            choice.value
        ))
        .text_size(13)
        .padding([2, 8])
        .width(LANGUAGE_PICKER_WIDTH),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Profile actions

    #[test]
    fn profile_menu_lists_every_action_once() {
        let messages: Vec<String> = ProfileAction::ALL
            .iter()
            .map(|action| format!("{:?}", action.message()))
            .collect();

        assert_eq!(
            messages,
            [
                "NewProfile",
                "SaveCurrentProfile",
                "ImportProfile",
                "ExportProfile",
                "OpenProfilesFolder"
            ]
        );
        assert_eq!(ProfileAction::Save.to_string(), tr(texts::PROFILE_SAVE));
    }
}

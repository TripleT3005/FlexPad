use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::canvas;
use crate::components::{self, Component};
use crate::config::{
    UserSettings, app_dirs, export_profile, get_profiles_dir, list_saved_profiles,
    load_profile_from_disk, load_user_settings, migrate_legacy_files, profile_file_stem,
    profile_path, read_profile_file, save_profile_to_disk, save_user_settings, unique_profile_name,
};
use crate::constants::PRESETS;
use crate::device::{
    Connection, DeviceEvent, FlexPadDeviceInfo, InputEvent, MacroTestResult, PinEdge,
    TurnDirection, connect_to_port, device_events, fetch_config_from_device, run_macro_on_device,
    scan_flexpad_devices, set_config_to_device,
};
use crate::file_dialog::{self, DialogError, DialogKind, DialogRequest, DialogResult};
use crate::flasher::{
    FlashEvent, FlashOutcome, FlashSource, FlashStage, embedded_image, find_download_port,
    update_firmware,
};
use crate::i18n::{self, Language, format_text, text as tr};
use crate::keys::KeyboardLayout;
use crate::macros::{Macro, MacroAction, Preset};
use crate::profile::{self, Profile, Tile};
use crate::texts;
use crate::theme;
use crate::ui::header::{view_header, view_status_bar};
use crate::ui::workspace::view_workspace;
use crate::update::{self, ReleaseInfo};
use crate::window_state::{self, WindowGeometry};

// UI state

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileItem {
    pub name: String,
    pub path: PathBuf,
}

impl std::fmt::Display for ProfileItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub struct UiState {
    pub selected_macro_key: String,
    pub macro_delay_input: String,
    pub selected_media_action: String,
    pub macro_text_input: String,
    pub selected_preset: String,
    pub advanced_mode: bool,
    pub expanded_functions: Vec<String>,
    pub raw_macro_input: Option<(String, String)>,
    pub pending_overwrite: bool,
    pub file_dialog_open: bool,
    pub highlighted_tiles: Vec<(String, Instant)>,
    pub macro_test: Option<MacroTest>,
}

impl UiState {
    pub fn highlighted_tile_ids(&self) -> Vec<String> {
        self.highlighted_tiles
            .iter()
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn macro_test_for(&self, tile_id: &str, function: &str) -> Option<&MacroTestState> {
        self.macro_test
            .as_ref()
            .filter(|test| test.tile_id == tile_id && test.function == function)
            .map(|test| &test.state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroTest {
    pub tile_id: String,
    pub function: String,
    pub state: MacroTestState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroTestState {
    Sending,
    Done(MacroTestResult),
    Failed(String),
}

impl std::fmt::Display for MacroTestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroTestState::Sending => f.write_str(&tr(texts::TEST_SENDING)),
            MacroTestState::Done(MacroTestResult::Queued) => f.write_str(&tr(texts::TEST_QUEUED)),
            MacroTestState::Done(MacroTestResult::Busy) => f.write_str(&tr(texts::TEST_BUSY)),
            MacroTestState::Done(MacroTestResult::Rejected(reason)) => {
                f.write_str(&format_text(texts::TEST_REJECTED, &[("reason", reason)]))
            }
            MacroTestState::Failed(error) => {
                f.write_str(&format_text(texts::TEST_FAILED, &[("error", error)]))
            }
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected_macro_key: "ControlLeft".to_string(),
            macro_delay_input: "50".to_string(),
            selected_media_action: "vol_up".to_string(),
            macro_text_input: "".to_string(),
            selected_preset: PRESETS.first().map(|p| p.0.to_string()).unwrap_or_default(),
            advanced_mode: false,
            expanded_functions: Vec::new(),
            raw_macro_input: None,
            pending_overwrite: false,
            file_dialog_open: false,
            highlighted_tiles: Vec::new(),
            macro_test: None,
        }
    }
}

// Constants

const HIGHLIGHT_DURATION: Duration = Duration::from_millis(400);
const HIGHLIGHT_TICK: Duration = Duration::from_millis(100);

// Device tasks

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTask {
    Scanning,
    Connecting,
    Loading,
    Saving,
    Flashing,
    TestingMacro,
}

impl DeviceTask {
    fn status(self) -> String {
        tr(match self {
            DeviceTask::Scanning => texts::TASK_SCANNING,
            DeviceTask::Connecting => texts::TASK_CONNECTING,
            DeviceTask::Loading => texts::TASK_LOADING,
            DeviceTask::Saving => texts::TASK_SAVING,
            DeviceTask::Flashing => texts::TASK_FLASHING,
            DeviceTask::TestingMacro => texts::TASK_TESTING_MACRO,
        })
    }
}

// App

pub struct FlexPadApp {
    pub status_message: String,
    pub theme_mode: iced::theme::Mode,
    pub available_devices: Vec<FlexPadDeviceInfo>,
    pub selected_device: Option<FlexPadDeviceInfo>,
    pub connected_device: Option<FlexPadDeviceInfo>,
    pub serial_port: Option<Connection>,
    pub device_task: Option<DeviceTask>,

    pub tiles: Vec<Tile>,
    pub selected_tile_id: Option<String>,
    pub dragging_tile_id: Option<String>,
    pub selected_component: Option<&'static Component>,

    pub current_profile_name: String,
    pub keyboard_layout: KeyboardLayout,
    pub saved_profiles: Vec<ProfileItem>,
    pub selected_profile: Option<ProfileItem>,

    pub check_for_updates: bool,
    pub available_update: Option<ReleaseInfo>,

    pub language: Option<String>,
    pub languages: Vec<Language>,

    pub flash_stage: Option<FlashStage>,
    pub download_port: Option<String>,

    pub ui: UiState,
}

// Messages

#[derive(Debug, Clone)]
pub enum Message {
    ScanDevices,
    DeviceSelected(FlexPadDeviceInfo),
    ConnectDevice,
    DisconnectDevice,
    LoadConfig,
    SetConfig,
    DevicesScanned(Vec<FlexPadDeviceInfo>, Option<String>),
    DeviceConnected(FlexPadDeviceInfo, Result<Connection, String>),
    ConfigLoaded(Result<String, String>),
    ConfigSet(Result<(), String>),
    DeviceTaskFailed,
    UpdateFirmware,
    Flash(FlashEvent),
    DeviceEvent(DeviceEvent),
    HighlightTick(Instant),
    CheckForUpdate,
    UpdateChecked(Result<Option<ReleaseInfo>, String>),
    UpdateCheckToggled(bool),
    LanguageSelected(Option<String>),
    CloseRequested(iced::window::Id),
    ThemeModeChanged(iced::theme::Mode),
    WindowRead(iced::window::Id, Option<WindowGeometry>),
    OpenReleasePage,
    ProfileSelected(ProfileItem),
    SaveCurrentProfile,
    NewProfile,
    ProfileNameChanged(String),
    ImportProfile,
    ExportProfile,
    OpenProfilesFolder,
    ImportFileChosen(DialogResult),
    ExportFileChosen(DialogResult),
    CanvasEvent(canvas::CanvasMessage),
    ComponentSelected(&'static Component),
    KeyboardLayoutSelected(KeyboardLayout),
    AddTile,
    SelectTile(String),
    UpdateTileName(String),
    UpdateTilePin(String, Option<u8>),
    UpdateParameter(String, serde_json::Value),
    ToggleFunctionSection(String),
    DeleteSelectedTile,
    MacroKeySelected(String),
    MacroDelayChanged(String),
    MediaActionSelected(String),
    MacroTextChanged(String),
    PresetSelected(String),
    AdvancedModeToggled(bool),
    RawMacroChanged(String, String),
    AddMacroAction(String, MacroAction),
    RemoveMacroAction(String, usize),
    MoveMacroActionUp(String, usize),
    MoveMacroActionDown(String, usize),
    ApplyPreset(String, Preset),
    TestMacro(String),
    MacroTested(String, String, Result<MacroTestResult, String>),
}

impl FlexPadApp {
    // Setup

    pub fn new() -> (Self, Task<Message>) {
        let moved_files = migrate_legacy_files();
        let profiles = load_profile_items();

        let user_settings = load_user_settings();
        i18n::select(user_settings.language.as_deref());
        let mut initial_profile = Profile::default();
        let mut initial_selected = None;
        let mut status_message = if moved_files > 0 {
            format_text(
                texts::MESSAGE_MOVED_FILES,
                &[
                    ("count", &moved_files.to_string()),
                    ("folder", &app_dirs().data.display().to_string()),
                ],
            )
        } else {
            ready_message()
        };

        let target_profile = user_settings
            .last_profile
            .and_then(|last| {
                let path = profile_path(&last);
                profiles.iter().find(|p| p.path == path).cloned()
            })
            .or_else(|| profiles.first().cloned());

        if let Some(item) = target_profile {
            match load_profile_from_disk(&item.path) {
                Ok(loaded) => {
                    initial_profile = loaded;
                    initial_selected = Some(item);
                }
                Err(e) => status_message = profile_load_failed(&item, &e),
            }
        }

        let ui_state = UiState {
            advanced_mode: user_settings.advanced_mode,
            ..UiState::default()
        };

        (
            Self {
                status_message,
                theme_mode: theme::mode_override().unwrap_or_default(),
                available_devices: Vec::new(),
                selected_device: None,
                connected_device: None,
                serial_port: None,
                device_task: None,
                tiles: initial_profile.tiles,
                selected_tile_id: None,
                dragging_tile_id: None,
                selected_component: Some(components::default_component()),
                current_profile_name: initial_profile.profile_name,
                keyboard_layout: initial_profile.keyboard_layout,
                saved_profiles: profiles,
                selected_profile: initial_selected,
                check_for_updates: user_settings.check_for_updates,
                available_update: None,
                language: user_settings.language,
                languages: i18n::available_languages(),
                flash_stage: None,
                download_port: None,
                ui: ui_state,
            },
            Task::batch([
                read_system_theme(),
                if user_settings.check_for_updates {
                    check_for_update()
                } else {
                    Task::none()
                },
            ]),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let events = match &self.serial_port {
            Some(connection) => {
                Subscription::run_with(connection.clone(), device_events).map(Message::DeviceEvent)
            }
            None => Subscription::none(),
        };
        let highlights = if self.ui.highlighted_tiles.is_empty() {
            Subscription::none()
        } else {
            iced::time::every(HIGHLIGHT_TICK).map(Message::HighlightTick)
        };
        let updates = if self.check_for_updates {
            iced::time::every(update::CHECK_INTERVAL).map(|_| Message::CheckForUpdate)
        } else {
            Subscription::none()
        };
        Subscription::batch([
            events,
            highlights,
            updates,
            iced::window::close_requests().map(Message::CloseRequested),
            if theme::mode_override().is_some() {
                Subscription::none()
            } else {
                iced::system::theme_changes().map(Message::ThemeModeChanged)
            },
        ])
    }

    // Editing

    pub fn selected_tile(&self) -> Option<&Tile> {
        let id = self.selected_tile_id.as_ref()?;
        self.tiles.iter().find(|t| t.id == *id)
    }

    fn selected_tile_mut(&mut self) -> Option<&mut Tile> {
        let id = self.selected_tile_id.clone()?;
        self.tiles.iter_mut().find(|t| t.id == id)
    }

    fn edit_macro(&mut self, function: &str, edit: impl FnOnce(&mut Macro)) {
        if let Some(tile) = self.selected_tile_mut() {
            edit(tile.functions.entry(function.to_string()).or_default());
        }
        if self
            .ui
            .macro_test
            .as_ref()
            .is_some_and(|test| test.function == function && test.state != MacroTestState::Sending)
        {
            self.ui.macro_test = None;
        }
    }

    fn edit_actions(&mut self, function: &str, edit: impl FnOnce(&mut Vec<MacroAction>)) {
        self.ui.raw_macro_input = None;
        self.edit_macro(function, |value| edit(&mut value.actions));
    }

    fn current_profile(&self) -> Profile {
        Profile {
            keyboard_layout: self.keyboard_layout,
            ..Profile::new(self.current_profile_name.clone(), self.tiles.clone())
        }
    }

    // Update

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanDevices => return self.handle_scan_devices(),
            Message::DeviceSelected(device) => self.selected_device = Some(device),
            Message::ConnectDevice => return self.handle_connect_device(),
            Message::DisconnectDevice => {
                if !self.is_device_busy() {
                    self.serial_port = None;
                    self.connected_device = None;
                    self.ui.highlighted_tiles.clear();
                    self.status_message = tr(texts::MESSAGE_DISCONNECTED);
                }
            }
            Message::LoadConfig => return self.handle_load_config(),
            Message::SetConfig => return self.handle_set_config(),
            Message::DevicesScanned(devices, download_port) => {
                self.handle_devices_scanned(devices, download_port)
            }
            Message::DeviceConnected(device, result) => {
                self.handle_device_connected(device, result)
            }
            Message::ConfigLoaded(result) => self.handle_config_loaded(result),
            Message::ConfigSet(result) => self.handle_config_set(result),
            Message::TestMacro(function) => return self.handle_test_macro(function),
            Message::MacroTested(tile_id, function, result) => {
                self.handle_macro_tested(tile_id, function, result)
            }
            Message::DeviceTaskFailed => {
                self.device_task = None;
                if let Some(test) = &mut self.ui.macro_test
                    && test.state == MacroTestState::Sending
                {
                    test.state = MacroTestState::Failed(tr(texts::ERROR_UNEXPECTED));
                }
                self.status_message = tr(texts::MESSAGE_ACCESS_FAILED);
            }
            Message::UpdateFirmware => return self.handle_update_firmware(),
            Message::Flash(FlashEvent::Progress(stage)) => self.flash_stage = Some(stage),
            Message::Flash(FlashEvent::Finished(result)) => {
                return self.handle_flash_finished(result);
            }
            Message::DeviceEvent(event) => self.handle_device_event(event, Instant::now()),
            Message::HighlightTick(now) => {
                self.ui.highlighted_tiles.retain(|(_, until)| *until > now);
            }
            Message::CheckForUpdate => return self.handle_check_for_update(),
            Message::UpdateChecked(result) => self.handle_update_checked(result),
            Message::UpdateCheckToggled(enabled) => {
                return self.handle_update_check_toggled(enabled);
            }
            Message::LanguageSelected(language) => self.handle_language_selected(language),
            Message::CloseRequested(id) => return read_window(id),
            Message::ThemeModeChanged(mode) => self.theme_mode = mode,
            Message::WindowRead(id, geometry) => {
                if let Some(geometry) = geometry {
                    window_state::save_window_geometry(&geometry);
                }
                return iced::window::close(id);
            }
            Message::OpenReleasePage => {
                if let Some(release) = &self.available_update {
                    update::open_in_browser(&release.url);
                }
            }
            Message::ProfileSelected(profile) => self.handle_profile_selected(profile),
            Message::SaveCurrentProfile => self.handle_save_profile(),
            Message::NewProfile => self.handle_new_profile(),
            Message::ProfileNameChanged(name) => {
                self.current_profile_name = name;
                self.ui.pending_overwrite = false;
            }
            Message::ImportProfile => return self.handle_import_profile(),
            Message::ExportProfile => return self.handle_export_profile(),
            Message::OpenProfilesFolder => {
                if let Err(e) = file_dialog::open_folder(&get_profiles_dir()) {
                    self.status_message =
                        format_text(texts::MESSAGE_FOLDER_OPEN_FAILED, &[("error", &e)]);
                }
            }
            Message::ImportFileChosen(result) => self.handle_import_file_chosen(result),
            Message::ExportFileChosen(result) => self.handle_export_file_chosen(result),
            Message::CanvasEvent(event) => self.handle_canvas_event(event),
            Message::ComponentSelected(component) => self.selected_component = Some(component),
            Message::KeyboardLayoutSelected(layout) => self.keyboard_layout = layout,
            Message::AddTile => self.handle_add_tile(),
            Message::SelectTile(id) => {
                self.selected_tile_id = Some(id);
                self.ui.expanded_functions.clear();
                self.ui.raw_macro_input = None;
            }
            Message::UpdateTileName(name) => {
                if let Some(tile) = self.selected_tile_mut() {
                    tile.display_name = name;
                }
            }
            Message::UpdateTilePin(role, pin) => {
                if let Some(tile) = self.selected_tile_mut() {
                    tile.set_pin(&role, pin);
                }
            }
            Message::UpdateParameter(id, value) => {
                if let Some(tile) = self.selected_tile_mut() {
                    let accepted = tile
                        .component()
                        .and_then(|component| component.parameter(&id))
                        .is_some_and(|parameter| parameter.accepts(&value));
                    if accepted {
                        tile.parameters.insert(id, value);
                    }
                }
            }
            Message::ToggleFunctionSection(function) => {
                if self.ui.expanded_functions.contains(&function) {
                    self.ui.expanded_functions.retain(|s| s != &function);
                } else {
                    self.ui.expanded_functions.push(function);
                }
            }
            Message::DeleteSelectedTile => {
                if let Some(id) = self.selected_tile_id.take() {
                    self.tiles.retain(|t| t.id != id);
                    self.ui.expanded_functions.clear();
                    self.ui.raw_macro_input = None;
                    self.status_message = tr(texts::MESSAGE_TILE_REMOVED);
                }
            }
            Message::MacroKeySelected(key) => self.ui.selected_macro_key = key,
            Message::MacroDelayChanged(val) => self.ui.macro_delay_input = val,
            Message::MediaActionSelected(action) => self.ui.selected_media_action = action,
            Message::MacroTextChanged(text_val) => self.ui.macro_text_input = text_val,
            Message::PresetSelected(preset_name) => self.ui.selected_preset = preset_name,
            Message::AdvancedModeToggled(val) => {
                self.ui.advanced_mode = val;
                self.persist_settings(self.current_profile_name.clone());
            }
            Message::RawMacroChanged(function, raw) => {
                if let Ok(parsed) = serde_json::from_str::<Macro>(&raw) {
                    self.edit_macro(&function, |value| *value = parsed);
                }
                self.ui.raw_macro_input = Some((function, raw));
            }
            Message::AddMacroAction(function, action) => {
                self.edit_actions(&function, |actions| actions.push(action));
            }
            Message::RemoveMacroAction(function, index) => {
                self.edit_actions(&function, |actions| {
                    if index < actions.len() {
                        actions.remove(index);
                    }
                });
            }
            Message::MoveMacroActionUp(function, index) => {
                self.edit_actions(&function, |actions| {
                    if index > 0 && index < actions.len() {
                        actions.swap(index, index - 1);
                    }
                });
            }
            Message::MoveMacroActionDown(function, index) => {
                self.edit_actions(&function, |actions| {
                    if index + 1 < actions.len() {
                        actions.swap(index, index + 1);
                    }
                });
            }
            Message::ApplyPreset(function, preset) => {
                self.ui.raw_macro_input = None;
                self.edit_macro(&function, |value| *value = preset.to_macro());
            }
        }
        Task::none()
    }

    // Device access

    fn start_device_task(&mut self, task: DeviceTask) -> bool {
        if self.is_device_busy() {
            return false;
        }
        self.device_task = Some(task);
        self.status_message = task.status().to_string();
        true
    }

    pub fn is_device_busy(&self) -> bool {
        self.device_task.is_some()
    }

    fn handle_scan_devices(&mut self) -> Task<Message> {
        if !self.start_device_task(DeviceTask::Scanning) {
            return Task::none();
        }
        in_background(
            || (scan_flexpad_devices(), find_download_port()),
            |(devices, download_port)| Message::DevicesScanned(devices, download_port),
        )
    }

    fn handle_devices_scanned(
        &mut self,
        devices: Vec<FlexPadDeviceInfo>,
        download_port: Option<String>,
    ) {
        self.download_port = download_port;
        self.device_task = None;
        self.available_devices = devices;
        if self.available_devices.is_empty() {
            self.status_message = match &self.download_port {
                Some(port) => format_text(
                    texts::MESSAGE_BOOTLOADER_FOUND,
                    &[("port", port), ("button", &tr(texts::FIRMWARE_INSTALL))],
                ),
                None => tr(texts::MESSAGE_NO_DEVICES),
            };
            self.selected_device = None;
        } else {
            self.status_message = format_text(
                texts::MESSAGE_DEVICES_FOUND,
                &[("count", &self.available_devices.len().to_string())],
            );
            if self.selected_device.is_none() {
                self.selected_device = self.available_devices.first().cloned();
            }
        }
    }

    fn handle_update_firmware(&mut self) -> Task<Message> {
        if self.is_device_busy() || embedded_image().is_none() {
            return Task::none();
        }
        let source = match (self.serial_port.clone(), self.download_port.clone()) {
            (Some(port), _) => FlashSource::Device(port),
            (None, Some(port_name)) => FlashSource::DownloadPort(port_name),
            (None, None) => {
                self.status_message = tr(texts::MESSAGE_NOT_CONNECTED);
                return Task::none();
            }
        };
        if !self.start_device_task(DeviceTask::Flashing) {
            return Task::none();
        }
        self.serial_port = None;
        self.connected_device = None;
        self.download_port = None;
        self.flash_stage = None;
        Task::run(flash_events(source), Message::Flash)
    }

    fn handle_flash_finished(&mut self, result: Result<FlashOutcome, String>) -> Task<Message> {
        self.device_task = None;
        self.flash_stage = None;
        match result {
            Ok(FlashOutcome::Restarted) => {
                let scan = self.handle_scan_devices();
                self.status_message = tr(texts::MESSAGE_FIRMWARE_RESTARTED);
                return scan;
            }
            Ok(FlashOutcome::NeedsManualReset) => {
                self.status_message = format_text(
                    texts::MESSAGE_FIRMWARE_MANUAL_RESET,
                    &[("button", &tr(texts::DEVICE_SCAN))],
                );
            }
            Err(e) => {
                self.status_message = format_text(
                    texts::MESSAGE_FIRMWARE_FAILED,
                    &[("error", &e), ("button", &tr(texts::DEVICE_SCAN))],
                );
            }
        }
        Task::none()
    }

    fn handle_connect_device(&mut self) -> Task<Message> {
        let Some(device) = self.selected_device.clone() else {
            return Task::none();
        };
        if !self.start_device_task(DeviceTask::Connecting) {
            return Task::none();
        }
        let port_name = device.port_name.clone();
        in_background(
            move || connect_to_port(&port_name),
            move |result| Message::DeviceConnected(device, result),
        )
    }

    fn handle_device_connected(
        &mut self,
        device: FlexPadDeviceInfo,
        result: Result<Connection, String>,
    ) {
        self.device_task = None;
        match result {
            Ok(port) => {
                self.status_message = format_text(
                    texts::MESSAGE_CONNECTED_TO,
                    &[("device", &device.device_id)],
                );
                self.serial_port = Some(port);
                self.connected_device = Some(device);
            }
            Err(e) => {
                self.status_message =
                    format_text(texts::MESSAGE_CONNECTION_FAILED, &[("error", &e)]);
            }
        }
    }

    fn handle_load_config(&mut self) -> Task<Message> {
        if self.is_device_busy() {
            return Task::none();
        }
        let Some(port) = self.serial_port.clone() else {
            self.status_message = tr(texts::MESSAGE_NOT_CONNECTED);
            return Task::none();
        };
        if !self.start_device_task(DeviceTask::Loading) {
            return Task::none();
        }
        in_background(
            move || fetch_config_from_device(&port),
            Message::ConfigLoaded,
        )
    }

    fn handle_config_loaded(&mut self, result: Result<String, String>) {
        self.device_task = None;
        let json_str = match result {
            Ok(json_str) => json_str,
            Err(e) => {
                self.serial_port = None;
                self.connected_device = None;
                self.status_message = format_text(texts::MESSAGE_LOAD_FAILED, &[("error", &e)]);
                return;
            }
        };
        let loaded = match profile::parse_profile(&json_str) {
            Ok(loaded) => loaded,
            Err(e) => {
                self.status_message =
                    format_text(texts::MESSAGE_CONFIG_UNREADABLE, &[("error", &e)]);
                return;
            }
        };

        let profile_name = if loaded.tiles.is_empty() || loaded.profile_name.is_empty() {
            self.connected_device
                .as_ref()
                .map(|dev| dev.device_id.clone())
                .unwrap_or_else(|| "FlexPad_Config".to_string())
        } else {
            loaded.profile_name.clone()
        };
        self.keyboard_layout = loaded.keyboard_layout;
        self.tiles = loaded.tiles;
        self.selected_tile_id = None;
        self.current_profile_name = profile_name.clone();
        self.ui.pending_overwrite = false;
        self.ui.expanded_functions.clear();
        self.ui.raw_macro_input = None;

        if let Ok(path) = save_profile_to_disk(&self.current_profile()) {
            self.saved_profiles = load_profile_items();
            self.selected_profile = self.saved_profiles.iter().find(|p| p.path == path).cloned();
        }

        self.persist_settings(profile_name);
        self.status_message = tr(texts::MESSAGE_CONFIG_LOADED);
    }

    fn handle_set_config(&mut self) -> Task<Message> {
        if self.is_device_busy() {
            return Task::none();
        }
        let json_str = match self.current_profile().to_json() {
            Ok(json_str) => json_str,
            Err(e) => {
                self.status_message =
                    format_text(texts::MESSAGE_CONFIG_SAVE_FAILED, &[("error", &e)]);
                return Task::none();
            }
        };
        let Some(port) = self.serial_port.clone() else {
            self.status_message = tr(texts::MESSAGE_NOT_CONNECTED);
            return Task::none();
        };
        if !self.start_device_task(DeviceTask::Saving) {
            return Task::none();
        }
        in_background(
            move || set_config_to_device(&port, &json_str),
            Message::ConfigSet,
        )
    }

    fn handle_test_macro(&mut self, function: String) -> Task<Message> {
        let Some(port) = self.serial_port.clone() else {
            self.status_message = tr(texts::MESSAGE_NOT_CONNECTED);
            return Task::none();
        };
        let Some(tile) = self.selected_tile() else {
            return Task::none();
        };
        let tile_id = tile.id.clone();
        let function_json = match serde_json::to_string(&Macro {
            actions: tile.macro_for(&function).to_vec(),
        }) {
            Ok(json) => json,
            Err(e) => {
                self.status_message = format_text(
                    texts::MESSAGE_TEST_PREPARE_FAILED,
                    &[("error", &e.to_string())],
                );
                return Task::none();
            }
        };
        if !self.start_device_task(DeviceTask::TestingMacro) {
            return Task::none();
        }
        self.ui.macro_test = Some(MacroTest {
            tile_id: tile_id.clone(),
            function: function.clone(),
            state: MacroTestState::Sending,
        });
        in_background(
            move || run_macro_on_device(&port, &function_json),
            move |result| Message::MacroTested(tile_id, function, result),
        )
    }

    fn handle_macro_tested(
        &mut self,
        tile_id: String,
        function: String,
        result: Result<MacroTestResult, String>,
    ) {
        self.device_task = None;
        let state = match result {
            Ok(result) => MacroTestState::Done(result),
            Err(e) => MacroTestState::Failed(e),
        };
        self.status_message = state.to_string();
        self.ui.macro_test = Some(MacroTest {
            tile_id,
            function,
            state,
        });
    }

    fn handle_config_set(&mut self, result: Result<(), String>) {
        self.device_task = None;
        self.status_message = match result {
            Ok(()) => tr(texts::MESSAGE_CONFIG_SAVED),
            Err(e) => format_text(texts::MESSAGE_CONFIG_SAVE_FAILED, &[("error", &e)]),
        };
    }

    // Live wiring

    fn handle_device_event(&mut self, event: DeviceEvent, now: Instant) {
        match event {
            DeviceEvent::Disconnected => {
                if self.serial_port.take().is_some() {
                    self.connected_device = None;
                    self.ui.highlighted_tiles.clear();
                    self.status_message = tr(texts::MESSAGE_UNPLUGGED);
                }
            }
            DeviceEvent::Input(input) => self.handle_input_event(input, now),
        }
    }

    fn handle_input_event(&mut self, event: InputEvent, now: Instant) {
        let tile = tile_for_input(&self.tiles, event);
        if let Some(status) = input_status(event, tile) {
            self.status_message = status;
        }
        if let Some(id) = tile.map(|tile| tile.id.clone()) {
            self.ui.highlighted_tiles.retain(|(other, _)| *other != id);
            self.ui
                .highlighted_tiles
                .push((id, now + HIGHLIGHT_DURATION));
        }
    }

    // Settings and update check

    fn persist_settings(&self, last_profile: String) {
        save_user_settings(&UserSettings {
            last_profile: Some(last_profile),
            advanced_mode: self.ui.advanced_mode,
            check_for_updates: self.check_for_updates,
            language: self.language.clone(),
        });
    }

    fn handle_language_selected(&mut self, language: Option<String>) {
        i18n::select(language.as_deref());
        self.language = language;
        self.languages = i18n::available_languages();
        self.persist_settings(self.current_profile_name.clone());
        if !self.is_device_busy() {
            self.status_message = if self.connected_device.is_some() {
                String::new()
            } else {
                ready_message()
            };
        }
    }

    fn handle_check_for_update(&mut self) -> Task<Message> {
        if self.check_for_updates {
            check_for_update()
        } else {
            Task::none()
        }
    }

    fn handle_update_checked(&mut self, result: Result<Option<ReleaseInfo>, String>) {
        if !self.check_for_updates {
            return;
        }
        if let Ok(release) = result {
            self.available_update = release;
        }
    }

    fn handle_update_check_toggled(&mut self, enabled: bool) -> Task<Message> {
        self.check_for_updates = enabled;
        self.persist_settings(self.current_profile_name.clone());
        if enabled {
            check_for_update()
        } else {
            self.available_update = None;
            Task::none()
        }
    }

    // Profiles

    fn handle_profile_selected(&mut self, item: ProfileItem) {
        match load_profile_from_disk(&item.path) {
            Ok(loaded) => {
                self.keyboard_layout = loaded.keyboard_layout;
                self.tiles = loaded.tiles;
                self.current_profile_name = loaded.profile_name.clone();
                self.selected_profile = Some(item);
                self.ui.pending_overwrite = false;
                self.selected_tile_id = None;
                self.ui.expanded_functions.clear();
                self.ui.raw_macro_input = None;

                self.persist_settings(loaded.profile_name);
                self.status_message = tr(texts::MESSAGE_PROFILE_LOADED);
            }
            Err(e) => {
                self.status_message = profile_load_failed(&item, &e);
            }
        }
    }

    fn handle_save_profile(&mut self) {
        let target_path = profile_path(&self.current_profile_name);
        let name_collision = self.saved_profiles.iter().any(|p| {
            p.path == target_path
                && Some(&p.path) != self.selected_profile.as_ref().map(|s| &s.path)
        });

        if name_collision && !self.ui.pending_overwrite {
            self.ui.pending_overwrite = true;
            self.status_message = format_text(
                texts::MESSAGE_PROFILE_EXISTS,
                &[
                    ("name", &self.current_profile_name),
                    ("button", &tr(texts::PROFILE_SAVE)),
                ],
            );
            return;
        }

        self.ui.pending_overwrite = false;

        if let Some(selected) = &self.selected_profile
            && selected.path != target_path
            && selected.path.exists()
        {
            let _ = std::fs::remove_file(&selected.path);
        }

        match save_profile_to_disk(&self.current_profile()) {
            Ok(path) => {
                self.saved_profiles = load_profile_items();
                self.selected_profile =
                    self.saved_profiles.iter().find(|p| p.path == path).cloned();

                self.persist_settings(self.current_profile_name.clone());
                self.status_message = tr(texts::MESSAGE_PROFILE_SAVED);
            }
            Err(e) => {
                self.status_message =
                    format_text(texts::MESSAGE_PROFILE_SAVE_FAILED, &[("error", &e)]);
            }
        }
    }

    fn handle_new_profile(&mut self) {
        let new_name = format!(
            "{}_{}",
            self.current_profile_name,
            tr(texts::PROFILE_COPY_SUFFIX)
        );
        self.current_profile_name = new_name.clone();
        self.ui.pending_overwrite = false;

        if let Ok(path) = save_profile_to_disk(&self.current_profile()) {
            self.saved_profiles = load_profile_items();
            self.selected_profile = self.saved_profiles.iter().find(|p| p.path == path).cloned();

            self.persist_settings(new_name.clone());
            self.status_message =
                format_text(texts::MESSAGE_PROFILE_CREATED, &[("name", &new_name)]);
        }
    }

    // Import and export

    fn handle_import_profile(&mut self) -> Task<Message> {
        self.open_file_dialog(
            DialogKind::Open,
            texts::DIALOG_IMPORT_TITLE,
            String::new(),
            Message::ImportFileChosen,
        )
    }

    fn handle_export_profile(&mut self) -> Task<Message> {
        let file_name = format!("{}.json", profile_file_stem(&self.current_profile_name));
        self.open_file_dialog(
            DialogKind::Save,
            texts::DIALOG_EXPORT_TITLE,
            file_name,
            Message::ExportFileChosen,
        )
    }

    fn open_file_dialog(
        &mut self,
        kind: DialogKind,
        title: &str,
        file_name: String,
        done: fn(DialogResult) -> Message,
    ) -> Task<Message> {
        if self.ui.file_dialog_open {
            return Task::none();
        }
        self.ui.file_dialog_open = true;
        let request = DialogRequest {
            kind,
            title: tr(title),
            filter_name: tr(texts::DIALOG_FILTER),
            file_name,
        };
        pick_file(request, done)
    }

    fn chosen_path(&mut self, result: DialogResult) -> Option<PathBuf> {
        self.ui.file_dialog_open = false;
        match result {
            Ok(path) => path,
            Err(DialogError::Unavailable) => {
                self.status_message = tr(texts::MESSAGE_DIALOG_UNAVAILABLE);
                None
            }
            Err(DialogError::Failed(e)) => {
                self.status_message = format_text(texts::MESSAGE_DIALOG_FAILED, &[("error", &e)]);
                None
            }
        }
    }

    fn handle_export_file_chosen(&mut self, result: DialogResult) {
        let Some(path) = self.chosen_path(result) else {
            return;
        };
        let path = file_dialog::with_json_extension(path);
        self.status_message = match export_profile(&self.current_profile(), &path) {
            Ok(()) => format_text(
                texts::MESSAGE_PROFILE_EXPORTED,
                &[("path", &path.display().to_string())],
            ),
            Err(e) => format_text(texts::MESSAGE_PROFILE_EXPORT_FAILED, &[("error", &e)]),
        };
    }

    fn handle_import_file_chosen(&mut self, result: DialogResult) {
        let Some(path) = self.chosen_path(result) else {
            return;
        };
        self.import_profile(&path);
    }

    fn import_profile(&mut self, path: &Path) {
        let file_name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let import_failed = |error: &str| {
            format_text(
                texts::MESSAGE_PROFILE_IMPORT_FAILED,
                &[("file", &file_name), ("error", error)],
            )
        };
        let mut imported = match read_profile_file(path) {
            Ok(imported) => imported,
            Err(e) => {
                self.status_message = import_failed(&e);
                return;
            }
        };
        let requested_name = if imported.profile_name.trim().is_empty() {
            path.file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        } else {
            imported.profile_name.trim().to_string()
        };
        imported.profile_name =
            unique_profile_name(&requested_name, |name| profile_path(name).exists());

        let saved_path = match save_profile_to_disk(&imported) {
            Ok(saved_path) => saved_path,
            Err(e) => {
                self.status_message = import_failed(&e);
                return;
            }
        };
        self.keyboard_layout = imported.keyboard_layout;
        self.tiles = imported.tiles;
        self.current_profile_name = imported.profile_name.clone();
        self.saved_profiles = load_profile_items();
        self.selected_profile = self
            .saved_profiles
            .iter()
            .find(|p| p.path == saved_path)
            .cloned();
        self.ui.pending_overwrite = false;
        self.selected_tile_id = None;
        self.ui.expanded_functions.clear();
        self.ui.raw_macro_input = None;
        self.persist_settings(imported.profile_name.clone());

        self.status_message = if imported.profile_name == requested_name {
            format_text(
                texts::MESSAGE_PROFILE_IMPORTED,
                &[("name", &imported.profile_name)],
            )
        } else {
            format_text(
                texts::MESSAGE_PROFILE_IMPORTED_RENAMED,
                &[
                    ("existing", &requested_name),
                    ("name", &imported.profile_name),
                ],
            )
        };
    }

    // Canvas

    fn handle_canvas_event(&mut self, event: canvas::CanvasMessage) {
        match event {
            canvas::CanvasMessage::SelectAndDrag(maybe_id) => {
                self.selected_tile_id = maybe_id.clone();
                self.dragging_tile_id = maybe_id;
                self.ui.expanded_functions.clear();
                self.ui.raw_macro_input = None;
            }
            canvas::CanvasMessage::DragPosition(x, y) => {
                if let Some(id) = &self.dragging_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                {
                    tile.x = x;
                    tile.y = y;
                }
            }
            canvas::CanvasMessage::EndDrag => {
                self.dragging_tile_id = None;
            }
        }
    }

    fn handle_add_tile(&mut self) {
        let component = self
            .selected_component
            .unwrap_or_else(components::default_component);
        let new_id = format!("tile_{}", uuid_short());
        let new_tile = Tile::new(
            component,
            new_id.clone(),
            format!("{} {}", component, self.tiles.len() + 1),
            60.0,
            60.0,
        );

        self.tiles.push(new_tile);
        self.selected_tile_id = Some(new_id);
        self.ui.expanded_functions.clear();
        self.ui.raw_macro_input = None;
        self.status_message = tr(texts::MESSAGE_TILE_ADDED);
    }

    // View

    pub fn theme(&self) -> iced::Theme {
        theme::app_theme(self.theme_mode)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = column![
            view_header(self),
            view_workspace(self),
            view_status_bar(self)
        ]
        .spacing(12);

        container(content.padding([12, 16]))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

// Helpers

fn ready_message() -> String {
    format_text(texts::MESSAGE_READY, &[("button", &tr(texts::DEVICE_SCAN))])
}

fn profile_load_failed(item: &ProfileItem, error: &str) -> String {
    format_text(
        texts::MESSAGE_PROFILE_LOAD_FAILED,
        &[("name", &item.name), ("error", error)],
    )
}

fn tile_for_input(tiles: &[Tile], event: InputEvent) -> Option<&Tile> {
    match event {
        InputEvent::Pin(pin, _) => tiles
            .iter()
            .find(|tile| tile.pins.values().any(|&used| used == pin)),
        InputEvent::Encoder(pin_a, _) => tiles.iter().find(|tile| tile.pin("a") == Some(pin_a)),
    }
}

fn input_status(event: InputEvent, tile: Option<&Tile>) -> Option<String> {
    let target = tile.map_or_else(
        || tr(texts::MESSAGE_NOT_ASSIGNED),
        |tile| tile.display_name.clone(),
    );
    let is_switch = tile.is_some_and(|tile| tile.kind == components::SWITCH_KIND);
    let (key, pin) = match event {
        InputEvent::Pin(pin, PinEdge::Pressed) if is_switch => (texts::MESSAGE_SWITCH_ON, pin),
        InputEvent::Pin(pin, PinEdge::Released) if is_switch => (texts::MESSAGE_SWITCH_OFF, pin),
        InputEvent::Pin(pin, PinEdge::Pressed) => (texts::MESSAGE_PIN_PRESSED, pin),
        InputEvent::Pin(_, PinEdge::Released) => return None,
        InputEvent::Encoder(pin_a, TurnDirection::Right) => (texts::MESSAGE_ENCODER_RIGHT, pin_a),
        InputEvent::Encoder(pin_a, TurnDirection::Left) => (texts::MESSAGE_ENCODER_LEFT, pin_a),
    };
    Some(format_text(
        key,
        &[("pin", &pin.to_string()), ("target", &target)],
    ))
}

fn load_profile_items() -> Vec<ProfileItem> {
    list_saved_profiles()
        .into_iter()
        .map(|(name, path)| ProfileItem { name, path })
        .collect()
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:x}", since_epoch.subsec_nanos())
}

fn check_for_update() -> Task<Message> {
    Task::perform(
        async { tokio::task::spawn_blocking(update::check_for_update).await },
        |result| Message::UpdateChecked(result.unwrap_or_else(|e| Err(e.to_string()))),
    )
}

fn flash_events(source: FlashSource) -> impl iced::futures::Stream<Item = FlashEvent> {
    iced::stream::channel(16, async move |mut output| {
        use iced::futures::SinkExt;
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let worker = tokio::task::spawn_blocking(move || {
            let progress = sender.clone();
            let result = update_firmware(source, &mut |stage| {
                let _ = progress.send(FlashEvent::Progress(stage));
            });
            let _ = sender.send(FlashEvent::Finished(result));
        });
        while let Some(event) = receiver.recv().await {
            let _ = output.send(event).await;
        }
        if let Err(e) = worker.await {
            let _ = output
                .send(FlashEvent::Finished(Err(format_text(
                    texts::ERROR_UPDATE_STOPPED,
                    &[("error", &e.to_string())],
                ))))
                .await;
        }
    })
}

fn read_system_theme() -> Task<Message> {
    if theme::mode_override().is_some() {
        Task::none()
    } else {
        iced::system::theme().map(Message::ThemeModeChanged)
    }
}

fn read_window(id: iced::window::Id) -> Task<Message> {
    iced::window::position(id).then(move |position| {
        iced::window::size(id).then(move |size| {
            iced::window::is_maximized(id).then(move |maximized| {
                iced::window::raw_id::<Message>(id).map(move |raw_id| {
                    let previous = window_state::load_window_geometry();
                    let geometry =
                        window_state::geometry(position, size, maximized, raw_id, previous);
                    Message::WindowRead(id, geometry)
                })
            })
        })
    })
}

fn pick_file(request: DialogRequest, done: fn(DialogResult) -> Message) -> Task<Message> {
    iced::window::latest()
        .then(|id| match id {
            Some(id) => iced::window::raw_id::<Message>(id).map(Some),
            None => Task::done(None),
        })
        .then(move |owner| {
            let request = request.clone();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || file_dialog::pick(&request, owner)).await
                },
                move |result| {
                    done(result.unwrap_or_else(|e| Err(DialogError::Failed(e.to_string()))))
                },
            )
        })
}

fn in_background<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) -> Message + Send + 'static,
) -> Task<Message> {
    Task::perform(
        async move { tokio::task::spawn_blocking(work).await },
        move |result| result.map_or(Message::DeviceTaskFailed, done),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> FlexPadApp {
        FlexPadApp {
            status_message: String::new(),
            theme_mode: iced::theme::Mode::None,
            available_devices: Vec::new(),
            selected_device: Some(device()),
            connected_device: None,
            serial_port: None,
            device_task: None,
            tiles: Vec::new(),
            selected_tile_id: None,
            dragging_tile_id: None,
            selected_component: None,
            current_profile_name: "Test".to_string(),
            keyboard_layout: KeyboardLayout::DEFAULT,
            saved_profiles: Vec::new(),
            selected_profile: None,
            check_for_updates: true,
            available_update: None,
            language: None,
            languages: Vec::new(),
            flash_stage: None,
            download_port: None,
            ui: UiState::default(),
        }
    }

    fn device() -> FlexPadDeviceInfo {
        FlexPadDeviceInfo {
            port_name: "COM9".to_string(),
            device_id: "FlexPad-0123456789AB".to_string(),
            firmware_version: Some("0.2.0".to_string()),
            protocol_version: 1,
        }
    }

    // Update notice

    fn release() -> ReleaseInfo {
        ReleaseInfo {
            version: "0.3.0".to_string(),
            url: "https://github.com/TripleT3005/FlexPad/releases/tag/v0.3.0".to_string(),
        }
    }

    #[test]
    fn a_found_release_is_kept_and_errors_leave_it() {
        let mut app = test_app();

        let _ = app.update(Message::UpdateChecked(Ok(Some(release()))));
        assert_eq!(app.available_update, Some(release()));

        let _ = app.update(Message::UpdateChecked(Err("offline".to_string())));
        assert_eq!(app.available_update, Some(release()));
    }

    #[test]
    fn switching_the_check_off_clears_the_notice_and_ignores_results() {
        let mut app = test_app();
        app.available_update = Some(release());
        app.check_for_updates = false;

        let _ = app.handle_check_for_update();
        let _ = app.update(Message::UpdateChecked(Ok(Some(release()))));
        assert_eq!(app.available_update, Some(release()));

        app.available_update = None;
        let _ = app.update(Message::UpdateChecked(Ok(Some(release()))));
        assert_eq!(app.available_update, None);
    }

    // Device access

    #[test]
    fn a_running_access_blocks_further_accesses() {
        let mut app = test_app();

        let _ = app.update(Message::ScanDevices);
        let _ = app.update(Message::ConnectDevice);
        let _ = app.update(Message::LoadConfig);
        let _ = app.update(Message::SetConfig);

        assert!(app.is_device_busy());
        assert_eq!(app.device_task, Some(DeviceTask::Scanning));
        assert_eq!(app.status_message, DeviceTask::Scanning.status());
    }

    #[test]
    fn success_ends_the_access() {
        let mut app = test_app();
        app.selected_device = None;

        let _ = app.update(Message::ScanDevices);
        let _ = app.update(Message::DevicesScanned(vec![device()], None));

        assert!(!app.is_device_busy());
        assert_eq!(app.available_devices, [device()]);
        assert_eq!(app.selected_device, Some(device()));
        assert_eq!(app.status_message, "Found 1 device(s).");
    }

    #[test]
    fn errors_end_the_access() {
        let mut app = test_app();

        let _ = app.update(Message::ConnectDevice);
        assert_eq!(app.device_task, Some(DeviceTask::Connecting));
        let _ = app.update(Message::DeviceConnected(
            device(),
            Err("Access denied".to_string()),
        ));
        assert!(!app.is_device_busy());
        assert!(app.connected_device.is_none());
        assert_eq!(app.status_message, "Connection failed: Access denied");

        app.device_task = Some(DeviceTask::Saving);
        let _ = app.update(Message::ConfigSet(Err("timeout".to_string())));
        assert!(!app.is_device_busy());
        assert_eq!(app.status_message, "Failed to save config: timeout");

        app.device_task = Some(DeviceTask::Loading);
        let _ = app.update(Message::ConfigLoaded(Err("timeout".to_string())));
        assert!(!app.is_device_busy());
        assert_eq!(app.status_message, "Error: timeout. Disconnected.");

        app.device_task = Some(DeviceTask::Loading);
        let _ = app.update(Message::ConfigLoaded(Ok("[]".to_string())));
        assert!(!app.is_device_busy());
        assert!(
            app.status_message
                .starts_with("Could not read the device configuration")
        );

        app.device_task = Some(DeviceTask::Scanning);
        let _ = app.update(Message::DeviceTaskFailed);
        assert!(!app.is_device_busy());
    }

    #[test]
    fn disconnecting_waits_for_a_running_access() {
        let mut app = test_app();
        app.connected_device = Some(device());
        app.device_task = Some(DeviceTask::Saving);

        let _ = app.update(Message::DisconnectDevice);
        assert!(app.connected_device.is_some());

        app.device_task = None;
        let _ = app.update(Message::DisconnectDevice);
        assert!(app.connected_device.is_none());
    }

    // Firmware update

    #[test]
    fn scanning_reports_a_board_in_bootloader_mode() {
        let mut app = test_app();

        let _ = app.update(Message::ScanDevices);
        let _ = app.update(Message::DevicesScanned(
            Vec::new(),
            Some("COM10".to_string()),
        ));

        assert_eq!(app.download_port.as_deref(), Some("COM10"));
        assert!(app.status_message.contains("bootloader mode is on COM10"));
    }

    #[test]
    fn firmware_update_tracks_progress_until_finished() {
        let mut app = test_app();
        app.device_task = Some(DeviceTask::Flashing);

        let stage = FlashStage::Writing {
            written: 1024,
            total: 4096,
        };
        let _ = app.update(Message::Flash(FlashEvent::Progress(stage)));
        assert_eq!(app.flash_stage, Some(stage));
        assert!(app.is_device_busy());

        let _ = app.update(Message::Flash(FlashEvent::Finished(Ok(
            FlashOutcome::NeedsManualReset,
        ))));
        assert!(!app.is_device_busy());
        assert_eq!(app.flash_stage, None);
        assert!(app.status_message.contains("Press RESET"));

        app.device_task = Some(DeviceTask::Flashing);
        let _ = app.update(Message::Flash(FlashEvent::Finished(Err(
            "the bootloader did not respond".to_string(),
        ))));
        assert!(!app.is_device_busy());
        assert!(
            app.status_message
                .starts_with("Firmware update failed: the bootloader did not respond")
        );

        app.device_task = Some(DeviceTask::Flashing);
        let _ = app.update(Message::Flash(FlashEvent::Finished(Ok(
            FlashOutcome::Restarted,
        ))));
        assert_eq!(app.device_task, Some(DeviceTask::Scanning));
        assert!(app.status_message.starts_with("Firmware updated."));
    }

    #[test]
    fn firmware_update_needs_a_device_and_an_idle_port() {
        let mut app = test_app();
        app.download_port = Some("COM10".to_string());
        app.device_task = Some(DeviceTask::Saving);

        let _ = app.update(Message::UpdateFirmware);
        assert_eq!(app.device_task, Some(DeviceTask::Saving));
        assert_eq!(app.download_port.as_deref(), Some("COM10"));

        app.device_task = None;
        app.download_port = None;
        let _ = app.update(Message::UpdateFirmware);
        assert!(!app.is_device_busy());
    }

    // Live wiring

    fn wired_app() -> FlexPadApp {
        let mut button = Tile::new(
            components::find("button").unwrap(),
            "copy".to_string(),
            "Copy".to_string(),
            0.0,
            0.0,
        );
        button.set_pin("switch", Some(4));
        let mut encoder = Tile::new(
            components::find("ec11").unwrap(),
            "volume".to_string(),
            "Volume".to_string(),
            0.0,
            0.0,
        );
        encoder.set_pin("a", Some(15));
        encoder.set_pin("b", Some(16));
        encoder.set_pin("switch", Some(17));
        let mut app = test_app();
        app.tiles = vec![button, encoder];
        app
    }

    fn input(app: &mut FlexPadApp, event: InputEvent, now: Instant) {
        app.handle_device_event(DeviceEvent::Input(event), now);
    }

    #[test]
    fn pin_events_highlight_their_tile() {
        let mut app = wired_app();
        let now = Instant::now();

        input(&mut app, InputEvent::Pin(4, PinEdge::Pressed), now);
        assert_eq!(app.ui.highlighted_tile_ids(), ["copy"]);
        assert_eq!(app.status_message, "GPIO 4 pressed – Copy");

        input(&mut app, InputEvent::Pin(17, PinEdge::Pressed), now);
        assert_eq!(app.ui.highlighted_tile_ids(), ["copy", "volume"]);
        assert_eq!(app.status_message, "GPIO 17 pressed – Volume");

        input(&mut app, InputEvent::Pin(4, PinEdge::Released), now);
        assert_eq!(app.ui.highlighted_tile_ids(), ["volume", "copy"]);
        assert_eq!(app.status_message, "GPIO 17 pressed – Volume");
    }

    #[test]
    fn switch_events_show_on_and_off() {
        let mut app = wired_app();
        let mut switch = Tile::new(
            components::find("toggle-switch").unwrap(),
            "light".to_string(),
            "Light".to_string(),
            0.0,
            0.0,
        );
        switch.set_pin("switch", Some(5));
        app.tiles.push(switch);
        let now = Instant::now();

        input(&mut app, InputEvent::Pin(5, PinEdge::Pressed), now);
        assert_eq!(app.ui.highlighted_tile_ids(), ["light"]);
        assert_eq!(app.status_message, "Switch on GPIO 5 turned on – Light");

        input(&mut app, InputEvent::Pin(5, PinEdge::Released), now);
        assert_eq!(app.ui.highlighted_tile_ids(), ["light"]);
        assert_eq!(app.status_message, "Switch on GPIO 5 turned off – Light");
    }

    #[test]
    fn encoder_events_use_pin_a() {
        let mut app = wired_app();
        let now = Instant::now();

        input(&mut app, InputEvent::Encoder(15, TurnDirection::Right), now);
        assert_eq!(app.ui.highlighted_tile_ids(), ["volume"]);
        assert_eq!(
            app.status_message,
            "Encoder on GPIO 15 turned right – Volume"
        );

        app.ui.highlighted_tiles.clear();
        input(&mut app, InputEvent::Encoder(16, TurnDirection::Left), now);
        assert!(app.ui.highlighted_tiles.is_empty());
        assert_eq!(
            app.status_message,
            "Encoder on GPIO 16 turned left – not assigned"
        );
    }

    #[test]
    fn unassigned_pins_only_show_in_the_status_line() {
        let mut app = wired_app();

        input(
            &mut app,
            InputEvent::Pin(12, PinEdge::Pressed),
            Instant::now(),
        );

        assert!(app.ui.highlighted_tiles.is_empty());
        assert_eq!(app.status_message, "GPIO 12 pressed – not assigned");
    }

    #[test]
    fn highlights_expire() {
        let mut app = wired_app();
        let now = Instant::now();

        input(&mut app, InputEvent::Pin(4, PinEdge::Pressed), now);
        let _ = app.update(Message::HighlightTick(now + HIGHLIGHT_DURATION / 2));
        assert_eq!(app.ui.highlighted_tile_ids(), ["copy"]);

        let _ = app.update(Message::HighlightTick(now + HIGHLIGHT_DURATION));
        assert!(app.ui.highlighted_tiles.is_empty());
    }

    #[test]
    fn unplugging_disconnects_and_clears_highlights() {
        let mut app = wired_app();
        app.serial_port = Some(Connection::open(UnpluggedLink));
        app.connected_device = Some(device());
        input(
            &mut app,
            InputEvent::Pin(4, PinEdge::Pressed),
            Instant::now(),
        );

        let _ = app.update(Message::DeviceEvent(DeviceEvent::Disconnected));

        assert!(app.serial_port.is_none());
        assert!(app.connected_device.is_none());
        assert!(app.ui.highlighted_tiles.is_empty());
        assert_eq!(app.status_message, "Device was unplugged! Disconnected.");
    }

    // Macro test

    fn app_ready_to_test() -> FlexPadApp {
        let mut app = wired_app();
        app.serial_port = Some(Connection::open(UnpluggedLink));
        app.connected_device = Some(device());
        app.selected_tile_id = Some("copy".to_string());
        app.tiles[0].functions.insert(
            "press".to_string(),
            Macro {
                actions: vec![MacroAction::Media("mute".to_string())],
            },
        );
        app
    }

    #[test]
    fn testing_needs_a_connection() {
        let mut app = app_ready_to_test();
        app.serial_port = None;

        let _ = app.update(Message::TestMacro("press".to_string()));

        assert!(!app.is_device_busy());
        assert!(app.ui.macro_test.is_none());
        assert_eq!(app.status_message, "Not connected to any device.");
    }

    #[test]
    fn test_results_are_shown_at_their_function() {
        let mut app = app_ready_to_test();

        let _ = app.update(Message::TestMacro("press".to_string()));
        assert_eq!(app.device_task, Some(DeviceTask::TestingMacro));
        assert_eq!(
            app.ui.macro_test_for("copy", "press"),
            Some(&MacroTestState::Sending)
        );

        let _ = app.update(Message::LoadConfig);
        assert_eq!(app.device_task, Some(DeviceTask::TestingMacro));

        let _ = app.update(Message::MacroTested(
            "copy".to_string(),
            "press".to_string(),
            Ok(MacroTestResult::Rejected("unknown_key:hyper".to_string())),
        ));
        assert!(!app.is_device_busy());
        assert_eq!(
            app.ui
                .macro_test_for("copy", "press")
                .map(ToString::to_string)
                .as_deref(),
            Some("Rejected by the device: unknown_key:hyper")
        );
        assert_eq!(app.ui.macro_test_for("volume", "press"), None);
        assert_eq!(app.ui.macro_test_for("copy", "turn_left"), None);
    }

    #[test]
    fn test_states_have_readable_labels() {
        assert_eq!(
            MacroTestState::Done(MacroTestResult::Queued).to_string(),
            "Macro queued on the device."
        );
        assert_eq!(
            MacroTestState::Done(MacroTestResult::Busy).to_string(),
            "The device is busy (macro queue full). Try again."
        );
        assert_eq!(
            MacroTestState::Failed("The device did not answer in time".to_string()).to_string(),
            "Test failed: The device did not answer in time"
        );
    }

    #[test]
    fn editing_the_macro_clears_its_test_result() {
        let mut app = app_ready_to_test();
        app.ui.macro_test = Some(MacroTest {
            tile_id: "copy".to_string(),
            function: "press".to_string(),
            state: MacroTestState::Done(MacroTestResult::Queued),
        });

        let _ = app.update(Message::AddMacroAction(
            "press".to_string(),
            MacroAction::Delay(10),
        ));

        assert!(app.ui.macro_test.is_none());
    }

    #[test]
    fn testing_does_not_change_the_profile() {
        let mut app = app_ready_to_test();
        let before = app.current_profile().to_json().unwrap();

        let _ = app.update(Message::TestMacro("press".to_string()));
        let _ = app.update(Message::MacroTested(
            "copy".to_string(),
            "press".to_string(),
            Ok(MacroTestResult::Queued),
        ));

        assert_eq!(app.current_profile().to_json().unwrap(), before);
        assert_eq!(app.status_message, "Macro queued on the device.");
    }

    struct UnpluggedLink;

    impl std::io::Read for UnpluggedLink {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }

    impl std::io::Write for UnpluggedLink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl crate::device::Link for UnpluggedLink {
        fn is_open(&self) -> bool {
            false
        }
    }
}

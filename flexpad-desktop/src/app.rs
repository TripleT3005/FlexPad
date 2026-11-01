use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task};
use std::path::PathBuf;
use std::time::Duration;

use crate::canvas;
use crate::components::{self, Component};
use crate::config::{
    UserSettings, app_dirs, list_saved_profiles, load_profile_from_disk, load_user_settings,
    migrate_legacy_files, profile_path, save_profile_to_disk, save_user_settings,
};
use crate::constants::PRESETS;
use crate::device::{
    FlexPadDeviceInfo, SharedPort, connect_to_port, fetch_config_from_device, scan_flexpad_devices,
    set_config_to_device,
};
use crate::flasher::{
    FlashEvent, FlashOutcome, FlashSource, FlashStage, embedded_image, find_download_port,
    update_firmware,
};
use crate::keys::KeyboardLayout;
use crate::macros::{Macro, MacroAction, Preset};
use crate::profile::{self, Profile, Tile};
use crate::ui::header::view_header;
use crate::ui::workspace::view_workspace;
use crate::update::{self, ReleaseInfo};

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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTask {
    Scanning,
    Connecting,
    Loading,
    Saving,
    Flashing,
}

impl DeviceTask {
    fn status(self) -> &'static str {
        match self {
            DeviceTask::Scanning => "Scanning for FlexPad devices...",
            DeviceTask::Connecting => "Connecting to the device...",
            DeviceTask::Loading => "Loading the configuration from the device...",
            DeviceTask::Saving => "Saving the configuration to the device...",
            DeviceTask::Flashing => "Updating the firmware...",
        }
    }
}

pub struct FlexPadApp {
    pub status_message: String,
    pub available_devices: Vec<FlexPadDeviceInfo>,
    pub selected_device: Option<FlexPadDeviceInfo>,
    pub connected_device: Option<FlexPadDeviceInfo>,
    pub serial_port: Option<SharedPort>,
    pub device_task: Option<DeviceTask>,

    pub tiles: Vec<Tile>,
    pub selected_tile_id: Option<String>,
    pub dragging_tile_id: Option<String>,
    pub selected_component: Option<Component>,

    pub current_profile_name: String,
    pub keyboard_layout: KeyboardLayout,
    pub saved_profiles: Vec<ProfileItem>,
    pub selected_profile: Option<ProfileItem>,

    pub check_for_updates: bool,
    pub available_update: Option<ReleaseInfo>,

    pub flash_stage: Option<FlashStage>,
    pub download_port: Option<String>,

    pub ui: UiState,
}

#[derive(Debug, Clone)]
pub enum Message {
    ScanDevices,
    DeviceSelected(FlexPadDeviceInfo),
    ConnectDevice,
    DisconnectDevice,
    LoadConfig,
    SetConfig,
    DevicesScanned(Vec<FlexPadDeviceInfo>, Option<String>),
    DeviceConnected(FlexPadDeviceInfo, Result<SharedPort, String>),
    ConfigLoaded(Result<String, String>),
    ConfigSet(Result<(), String>),
    DeviceTaskFailed,
    UpdateFirmware,
    Flash(FlashEvent),
    CheckConnectionTick,
    CheckForUpdate,
    UpdateChecked(Result<Option<ReleaseInfo>, String>),
    UpdateCheckToggled(bool),
    OpenReleasePage,
    ProfileSelected(ProfileItem),
    SaveCurrentProfile,
    NewProfile,
    ProfileNameChanged(String),
    CanvasEvent(canvas::CanvasMessage),
    ComponentSelected(Component),
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
}

impl FlexPadApp {
    pub fn new() -> (Self, Task<Message>) {
        let moved_files = migrate_legacy_files();
        let profiles = load_profile_items();

        let user_settings = load_user_settings();
        let mut initial_profile = Profile::default();
        let mut initial_selected = None;
        let mut status_message = if moved_files > 0 {
            format!(
                "Moved {moved_files} file(s) from the old FlexPad folder to {}.",
                app_dirs().data.display()
            )
        } else {
            "Ready. Click 'Scan' to search for devices.".to_string()
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
                Err(e) => status_message = format!("Could not load profile '{}': {}", item.name, e),
            }
        }

        let ui_state = UiState {
            advanced_mode: user_settings.advanced_mode,
            ..UiState::default()
        };

        (
            Self {
                status_message,
                available_devices: Vec::new(),
                selected_device: None,
                connected_device: None,
                serial_port: None,
                device_task: None,
                tiles: initial_profile.tiles,
                selected_tile_id: None,
                dragging_tile_id: None,
                selected_component: Some(components::BUTTON),
                current_profile_name: initial_profile.profile_name,
                keyboard_layout: initial_profile.keyboard_layout,
                saved_profiles: profiles,
                selected_profile: initial_selected,
                check_for_updates: user_settings.check_for_updates,
                available_update: None,
                flash_stage: None,
                download_port: None,
                ui: ui_state,
            },
            if user_settings.check_for_updates {
                check_for_update()
            } else {
                Task::none()
            },
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let connection =
            iced::time::every(Duration::from_secs(1)).map(|_| Message::CheckConnectionTick);
        if self.check_for_updates {
            let updates =
                iced::time::every(update::CHECK_INTERVAL).map(|_| Message::CheckForUpdate);
            Subscription::batch([connection, updates])
        } else {
            connection
        }
    }

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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanDevices => return self.handle_scan_devices(),
            Message::DeviceSelected(device) => self.selected_device = Some(device),
            Message::ConnectDevice => return self.handle_connect_device(),
            Message::DisconnectDevice => {
                if !self.is_device_busy() {
                    self.serial_port = None;
                    self.connected_device = None;
                    self.status_message = "Disconnected.".to_string();
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
            Message::DeviceTaskFailed => {
                self.device_task = None;
                self.status_message = "The device access failed unexpectedly.".to_string();
            }
            Message::UpdateFirmware => return self.handle_update_firmware(),
            Message::Flash(FlashEvent::Progress(stage)) => self.flash_stage = Some(stage),
            Message::Flash(FlashEvent::Finished(result)) => {
                return self.handle_flash_finished(result);
            }
            Message::CheckConnectionTick => self.handle_connection_tick(),
            Message::CheckForUpdate => return self.handle_check_for_update(),
            Message::UpdateChecked(result) => self.handle_update_checked(result),
            Message::UpdateCheckToggled(enabled) => {
                return self.handle_update_check_toggled(enabled);
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
                    self.status_message = "Tile removed from canvas.".to_string();
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
                Some(port) => format!(
                    "No FlexPad found, but a board in bootloader mode is on {port}. \
                     Click 'Install firmware' to set it up."
                ),
                None => "No FlexPad devices found.".to_string(),
            };
            self.selected_device = None;
        } else {
            self.status_message = format!("Found {} device(s).", self.available_devices.len());
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
                self.status_message = "Not connected to any device.".to_string();
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
                self.status_message =
                    "Firmware updated. The FlexPad restarted, scanning for it...".to_string();
                return scan;
            }
            Ok(FlashOutcome::NeedsManualReset) => {
                self.status_message = "Firmware updated. Press RESET on the FlexPad (or unplug \
                                       and replug it), then click 'Scan Devices'."
                    .to_string();
            }
            Err(e) => {
                self.status_message = format!(
                    "Firmware update failed: {e}. Click 'Scan Devices' and try again; \
                     your configuration is kept."
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
            move || connect_to_port(&port_name).map(SharedPort::new),
            move |result| Message::DeviceConnected(device, result),
        )
    }

    fn handle_device_connected(
        &mut self,
        device: FlexPadDeviceInfo,
        result: Result<SharedPort, String>,
    ) {
        self.device_task = None;
        match result {
            Ok(port) => {
                self.status_message = format!("Connected to {}", device.device_id);
                self.serial_port = Some(port);
                self.connected_device = Some(device);
            }
            Err(e) => {
                self.status_message = format!("Connection failed: {}", e);
            }
        }
    }

    fn handle_load_config(&mut self) -> Task<Message> {
        if self.is_device_busy() {
            return Task::none();
        }
        let Some(port) = self.serial_port.clone() else {
            self.status_message = "Not connected to any device.".to_string();
            return Task::none();
        };
        if !self.start_device_task(DeviceTask::Loading) {
            return Task::none();
        }
        in_background(
            move || fetch_config_from_device(&mut port.lock()),
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
                self.status_message = format!("Error: {}. Disconnected.", e);
                return;
            }
        };
        let loaded = match profile::parse_profile(&json_str) {
            Ok(loaded) => loaded,
            Err(e) => {
                self.status_message = format!("Could not read the device configuration: {}", e);
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
        self.status_message = "Configuration loaded from device and saved locally.".to_string();
    }

    fn handle_set_config(&mut self) -> Task<Message> {
        if self.is_device_busy() {
            return Task::none();
        }
        let json_str = match self.current_profile().to_json() {
            Ok(json_str) => json_str,
            Err(e) => {
                self.status_message = format!("Failed to save config: {}", e);
                return Task::none();
            }
        };
        let Some(port) = self.serial_port.clone() else {
            self.status_message = "Not connected to any device.".to_string();
            return Task::none();
        };
        if !self.start_device_task(DeviceTask::Saving) {
            return Task::none();
        }
        in_background(
            move || set_config_to_device(&mut port.lock(), &json_str),
            Message::ConfigSet,
        )
    }

    fn handle_config_set(&mut self, result: Result<(), String>) {
        self.device_task = None;
        self.status_message = match result {
            Ok(()) => "Configuration saved to device.".to_string(),
            Err(e) => format!("Failed to save config: {}", e),
        };
    }

    fn handle_connection_tick(&mut self) {
        if self.is_device_busy() {
            return;
        }
        let unplugged = self
            .serial_port
            .as_ref()
            .and_then(SharedPort::try_lock)
            .is_some_and(|port| port.clear(serialport::ClearBuffer::Input).is_err());
        if unplugged {
            self.serial_port = None;
            self.connected_device = None;
            self.status_message = "Device was unplugged! Disconnected.".to_string();
        }
    }

    fn persist_settings(&self, last_profile: String) {
        save_user_settings(&UserSettings {
            last_profile: Some(last_profile),
            advanced_mode: self.ui.advanced_mode,
            check_for_updates: self.check_for_updates,
        });
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
                self.status_message = "Loaded profile from disk.".to_string();
            }
            Err(e) => {
                self.status_message = format!("Could not load profile '{}': {}", item.name, e);
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
            self.status_message = format!(
                "Warning: Profile '{}' already exists. Click 'Save Profile' again to overwrite.",
                self.current_profile_name
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
                self.status_message = "Profile successfully saved/renamed to disk.".to_string();
            }
            Err(e) => {
                self.status_message = format!("Failed to save profile: {}", e);
            }
        }
    }

    fn handle_new_profile(&mut self) {
        let new_name = format!("{}_Copy", self.current_profile_name);
        self.current_profile_name = new_name.clone();
        self.ui.pending_overwrite = false;

        if let Ok(path) = save_profile_to_disk(&self.current_profile()) {
            self.saved_profiles = load_profile_items();
            self.selected_profile = self.saved_profiles.iter().find(|p| p.path == path).cloned();

            self.persist_settings(new_name.clone());
            self.status_message = format!("Created profile template: {}", new_name);
        }
    }

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
        let component = self.selected_component.unwrap_or(components::BUTTON);
        let new_id = format!("tile_{}", uuid_short());
        let new_tile = Tile::new(
            &component,
            new_id.clone(),
            format!("{} {}", component.name, self.tiles.len() + 1),
            60.0,
            60.0,
        );

        self.tiles.push(new_tile);
        self.selected_tile_id = Some(new_id);
        self.ui.expanded_functions.clear();
        self.ui.raw_macro_input = None;
        self.status_message = "Added new tile to canvas.".to_string();
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = view_header(self);

        let workspace = view_workspace(self);

        let content = column![header, workspace,].spacing(15);

        container(content.padding(20))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
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
                .send(FlashEvent::Finished(Err(format!(
                    "the update stopped: {e}"
                ))))
                .await;
        }
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
            keyboard_layout: KeyboardLayout::Us,
            saved_profiles: Vec::new(),
            selected_profile: None,
            check_for_updates: true,
            available_update: None,
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
}

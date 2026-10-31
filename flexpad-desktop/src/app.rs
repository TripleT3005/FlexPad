use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task};
use std::path::PathBuf;
use std::time::Duration;

use crate::canvas;
use crate::components::{self, Component};
use crate::config::{
    MacroAction, UserSettings, compile_macro, list_saved_profiles, load_profile_from_disk,
    load_user_settings, parse_macro, save_profile_to_disk, save_user_settings,
};
use crate::constants::PRESETS;
use crate::device::{
    FlexPadDeviceInfo, connect_to_port, fetch_config_from_device, scan_flexpad_devices,
    set_config_to_device,
};
use crate::profile::{self, Profile, Tile};
use crate::ui::header::view_header;
use crate::ui::workspace::view_workspace;

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
    pub pending_overwrite: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected_macro_key: "lctrl".to_string(),
            macro_delay_input: "50".to_string(),
            selected_media_action: "vol_up".to_string(),
            macro_text_input: "".to_string(),
            selected_preset: PRESETS.first().map(|p| p.0.to_string()).unwrap_or_default(),
            advanced_mode: false,
            expanded_functions: Vec::new(),
            pending_overwrite: false,
        }
    }
}

pub struct FlexPadApp {
    pub status_message: String,
    pub available_devices: Vec<FlexPadDeviceInfo>,
    pub selected_device: Option<FlexPadDeviceInfo>,
    pub connected_device: Option<FlexPadDeviceInfo>,
    pub serial_port: Option<Box<dyn serialport::SerialPort>>,

    pub tiles: Vec<Tile>,
    pub selected_tile_id: Option<String>,
    pub dragging_tile_id: Option<String>,
    pub selected_component: Option<Component>,

    pub current_profile_name: String,
    pub saved_profiles: Vec<ProfileItem>,
    pub selected_profile: Option<ProfileItem>,

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
    CheckConnectionTick,
    ProfileSelected(ProfileItem),
    SaveCurrentProfile,
    NewProfile,
    ProfileNameChanged(String),
    CanvasEvent(canvas::CanvasMessage),
    ComponentSelected(Component),
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
    RawMacroStringChanged(String, String),
    AddMacroAction(String, MacroAction),
    RemoveMacroAction(String, usize),
    MoveMacroActionUp(String, usize),
    MoveMacroActionDown(String, usize),
    ApplyPreset(String, String),
}

impl FlexPadApp {
    pub fn new() -> (Self, Task<Message>) {
        let profiles = load_profile_items();

        let user_settings = load_user_settings();
        let mut initial_profile = Profile::default();
        let mut initial_selected = None;
        let mut status_message = "Ready. Click 'Scan' to search for devices.".to_string();

        let target_profile = user_settings
            .last_profile
            .and_then(|last| profiles.iter().find(|p| p.name == last).cloned())
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
                tiles: initial_profile.tiles,
                selected_tile_id: None,
                dragging_tile_id: None,
                selected_component: Some(components::BUTTON),
                current_profile_name: initial_profile.profile_name,
                saved_profiles: profiles,
                selected_profile: initial_selected,
                ui: ui_state,
            },
            Task::none(),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_secs(1)).map(|_| Message::CheckConnectionTick)
    }

    pub fn selected_tile(&self) -> Option<&Tile> {
        let id = self.selected_tile_id.as_ref()?;
        self.tiles.iter().find(|t| t.id == *id)
    }

    fn selected_tile_mut(&mut self) -> Option<&mut Tile> {
        let id = self.selected_tile_id.clone()?;
        self.tiles.iter_mut().find(|t| t.id == id)
    }

    fn edit_macro(&mut self, function: &str, edit: impl FnOnce(&mut String)) {
        if let Some(tile) = self.selected_tile_mut() {
            edit(tile.functions.entry(function.to_string()).or_default());
        }
    }

    fn edit_actions(&mut self, function: &str, edit: impl FnOnce(&mut Vec<MacroAction>)) {
        self.edit_macro(function, |macro_str| {
            let mut actions = parse_macro(macro_str);
            edit(&mut actions);
            *macro_str = compile_macro(&actions);
        });
    }

    fn current_profile(&self) -> Profile {
        Profile::new(self.current_profile_name.clone(), self.tiles.clone())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanDevices => self.handle_scan_devices(),
            Message::DeviceSelected(device) => self.selected_device = Some(device),
            Message::ConnectDevice => self.handle_connect_device(),
            Message::DisconnectDevice => {
                self.serial_port = None;
                self.connected_device = None;
                self.status_message = "Disconnected.".to_string();
            }
            Message::LoadConfig => self.handle_load_config(),
            Message::SetConfig => self.handle_set_config(),
            Message::CheckConnectionTick => self.handle_connection_tick(),
            Message::ProfileSelected(profile) => self.handle_profile_selected(profile),
            Message::SaveCurrentProfile => self.handle_save_profile(),
            Message::NewProfile => self.handle_new_profile(),
            Message::ProfileNameChanged(name) => {
                self.current_profile_name = name;
                self.ui.pending_overwrite = false;
            }
            Message::CanvasEvent(event) => self.handle_canvas_event(event),
            Message::ComponentSelected(component) => self.selected_component = Some(component),
            Message::AddTile => self.handle_add_tile(),
            Message::SelectTile(id) => {
                self.selected_tile_id = Some(id);
                self.ui.expanded_functions.clear();
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
                save_user_settings(&UserSettings {
                    last_profile: Some(self.current_profile_name.clone()),
                    advanced_mode: self.ui.advanced_mode,
                });
            }
            Message::RawMacroStringChanged(function, raw) => {
                self.edit_macro(&function, |macro_str| *macro_str = raw);
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
            Message::ApplyPreset(function, macro_str) => {
                self.edit_macro(&function, |current| *current = macro_str);
            }
        }
        Task::none()
    }

    fn handle_scan_devices(&mut self) {
        self.status_message = "Scanning for FlexPad devices...".to_string();
        self.available_devices = scan_flexpad_devices();
        if self.available_devices.is_empty() {
            self.status_message = "No FlexPad devices found.".to_string();
            self.selected_device = None;
        } else {
            self.status_message = format!("Found {} device(s).", self.available_devices.len());
            if self.selected_device.is_none() {
                self.selected_device = self.available_devices.first().cloned();
            }
        }
    }

    fn handle_connect_device(&mut self) {
        if let Some(device) = &self.selected_device {
            match connect_to_port(&device.port_name) {
                Ok(port) => {
                    self.serial_port = Some(port);
                    self.connected_device = Some(device.clone());
                    self.status_message = format!("Connected to {}", device.device_id);
                }
                Err(e) => {
                    self.status_message = format!("Connection failed: {}", e);
                }
            }
        }
    }

    fn handle_load_config(&mut self) {
        let Some(port) = &mut self.serial_port else {
            self.status_message = "Not connected to any device.".to_string();
            return;
        };
        let json_str = match fetch_config_from_device(port) {
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
        self.tiles = loaded.tiles;
        self.selected_tile_id = None;
        self.current_profile_name = profile_name.clone();
        self.ui.pending_overwrite = false;
        self.ui.expanded_functions.clear();

        if save_profile_to_disk(&self.current_profile()).is_ok() {
            self.saved_profiles = load_profile_items();
            self.selected_profile = self
                .saved_profiles
                .iter()
                .find(|p| p.name == profile_name)
                .cloned();
        }

        save_user_settings(&UserSettings {
            last_profile: Some(profile_name),
            advanced_mode: self.ui.advanced_mode,
        });
        self.status_message = "Configuration loaded from device and saved locally.".to_string();
    }

    fn handle_set_config(&mut self) {
        let json_str = match self.current_profile().to_json() {
            Ok(json_str) => json_str,
            Err(e) => {
                self.status_message = format!("Failed to save config: {}", e);
                return;
            }
        };
        let Some(port) = &mut self.serial_port else {
            self.status_message = "Not connected to any device.".to_string();
            return;
        };
        self.status_message = match set_config_to_device(port, &json_str) {
            Ok(_) => "Configuration saved to device.".to_string(),
            Err(e) => format!("Failed to save config: {}", e),
        };
    }

    fn handle_connection_tick(&mut self) {
        if let Some(port) = &mut self.serial_port
            && port.clear(serialport::ClearBuffer::Input).is_err()
        {
            self.serial_port = None;
            self.connected_device = None;
            self.status_message = "Device was unplugged! Disconnected.".to_string();
        }
    }

    fn handle_profile_selected(&mut self, item: ProfileItem) {
        match load_profile_from_disk(&item.path) {
            Ok(loaded) => {
                self.tiles = loaded.tiles;
                self.current_profile_name = loaded.profile_name.clone();
                self.selected_profile = Some(item);
                self.ui.pending_overwrite = false;
                self.selected_tile_id = None;
                self.ui.expanded_functions.clear();

                save_user_settings(&UserSettings {
                    last_profile: Some(loaded.profile_name),
                    advanced_mode: self.ui.advanced_mode,
                });
                self.status_message = "Loaded profile from disk.".to_string();
            }
            Err(e) => {
                self.status_message = format!("Could not load profile '{}': {}", item.name, e);
            }
        }
    }

    fn handle_save_profile(&mut self) {
        let name_collision = self.saved_profiles.iter().any(|p| {
            p.name == self.current_profile_name
                && Some(p.path.clone()) != self.selected_profile.as_ref().map(|s| s.path.clone())
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
            && selected.name != self.current_profile_name
            && selected.path.exists()
        {
            let _ = std::fs::remove_file(&selected.path);
        }

        match save_profile_to_disk(&self.current_profile()) {
            Ok(_) => {
                self.saved_profiles = load_profile_items();
                self.selected_profile = self
                    .saved_profiles
                    .iter()
                    .find(|p| p.name == self.current_profile_name)
                    .cloned();

                save_user_settings(&UserSettings {
                    last_profile: Some(self.current_profile_name.clone()),
                    advanced_mode: self.ui.advanced_mode,
                });
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

        if save_profile_to_disk(&self.current_profile()).is_ok() {
            self.saved_profiles = load_profile_items();
            self.selected_profile = self
                .saved_profiles
                .iter()
                .find(|p| p.name == new_name)
                .cloned();

            save_user_settings(&UserSettings {
                last_profile: Some(new_name.clone()),
                advanced_mode: self.ui.advanced_mode,
            });
            self.status_message = format!("Created profile template: {}", new_name);
        }
    }

    fn handle_canvas_event(&mut self, event: canvas::CanvasMessage) {
        match event {
            canvas::CanvasMessage::SelectAndDrag(maybe_id) => {
                self.selected_tile_id = maybe_id.clone();
                self.dragging_tile_id = maybe_id;
                self.ui.expanded_functions.clear();
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
        self.status_message = "Added new tile to canvas.".to_string();
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = view_header(
            &self.status_message,
            &self.available_devices,
            self.selected_device.as_ref(),
            self.connected_device.as_ref(),
            &self.saved_profiles,
            self.selected_profile.as_ref(),
            &self.current_profile_name,
        );

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

use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::canvas;
use crate::config::{
    CanvasTile, FlexPadConfig, MacroAction, TileFunction, TileType, UserSettings, compile_macro,
    list_saved_profiles, load_profile_from_disk, load_user_settings, parse_macro,
    save_profile_to_disk, save_user_settings,
};
use crate::constants::PRESETS;
use crate::device::{
    FlexPadDeviceInfo, connect_to_port, fetch_config_from_device, scan_flexpad_devices,
    set_config_to_device,
};
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
    pub expanded_slots: Vec<String>,
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
            expanded_slots: Vec::new(),
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

    pub tiles: Vec<CanvasTile>,
    pub selected_tile_id: Option<String>,
    pub dragging_tile_id: Option<String>,
    pub selected_tile_type: Option<TileType>,

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
    TileTypeSelected(TileType),
    AddTile,
    SelectTile(String),
    UpdateTileName(String),
    UpdateTilePin(String, String),
    ToggleSlotSection(String),
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
        let raw_profiles = list_saved_profiles();
        let profiles: Vec<ProfileItem> = raw_profiles
            .into_iter()
            .map(|(name, path)| ProfileItem { name, path })
            .collect();

        let user_settings = load_user_settings();
        let mut initial_config = FlexPadConfig::default();
        let mut current_name = "Default_Profile".to_string();
        let mut initial_selected = None;

        let target_profile = user_settings
            .last_profile
            .and_then(|last| profiles.iter().find(|p| p.name == last).cloned())
            .or_else(|| profiles.first().cloned());

        if let Some(profile) = target_profile
            && let Ok(cfg) = load_profile_from_disk(&profile.path)
        {
            current_name = cfg.profile_name.clone();
            initial_config = cfg;
            initial_selected = Some(profile);
        }

        let ui_state = UiState {
            advanced_mode: user_settings.advanced_mode,
            ..UiState::default()
        };

        (
            Self {
                status_message: "Ready. Click 'Scan' to search for devices.".to_string(),
                available_devices: Vec::new(),
                selected_device: None,
                connected_device: None,
                serial_port: None,
                tiles: initial_config.tiles,
                selected_tile_id: None,
                dragging_tile_id: None,
                selected_tile_type: Some(TileType::Button),
                current_profile_name: current_name,
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanDevices => {
                self.handle_scan_devices();
                Task::none()
            }
            Message::DeviceSelected(device) => {
                self.selected_device = Some(device);
                Task::none()
            }
            Message::ConnectDevice => {
                self.handle_connect_device();
                Task::none()
            }
            Message::DisconnectDevice => {
                self.serial_port = None;
                self.connected_device = None;
                self.status_message = "Disconnected.".to_string();
                Task::none()
            }
            Message::LoadConfig => {
                self.handle_load_config();
                Task::none()
            }
            Message::SetConfig => {
                self.handle_set_config();
                Task::none()
            }
            Message::CheckConnectionTick => {
                self.handle_connection_tick();
                Task::none()
            }
            Message::ProfileSelected(profile) => {
                self.handle_profile_selected(profile);
                Task::none()
            }
            Message::SaveCurrentProfile => self.handle_save_profile(),
            Message::NewProfile => {
                self.handle_new_profile();
                Task::none()
            }
            Message::ProfileNameChanged(name) => {
                self.current_profile_name = name;
                self.ui.pending_overwrite = false;
                Task::none()
            }
            Message::CanvasEvent(event) => {
                self.handle_canvas_event(event);
                Task::none()
            }
            Message::TileTypeSelected(t_type) => {
                self.selected_tile_type = Some(t_type);
                Task::none()
            }
            Message::AddTile => {
                self.handle_add_tile();
                Task::none()
            }
            Message::SelectTile(id) => {
                self.selected_tile_id = Some(id);
                self.ui.expanded_slots.clear();
                Task::none()
            }
            Message::UpdateTileName(name) => {
                if let Some(id) = &self.selected_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                {
                    tile.display_name = name;
                }
                Task::none()
            }
            Message::UpdateTilePin(slot_key, new_pin) => {
                if let Some(id) = &self.selected_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                    && let Some(func) = tile.functions.get_mut(&slot_key)
                {
                    func.pin = new_pin;
                }
                Task::none()
            }
            Message::ToggleSlotSection(slot_key) => {
                if self.ui.expanded_slots.contains(&slot_key) {
                    self.ui.expanded_slots.retain(|s| s != &slot_key);
                } else {
                    self.ui.expanded_slots.push(slot_key);
                }
                Task::none()
            }
            Message::DeleteSelectedTile => {
                if let Some(id) = self.selected_tile_id.take() {
                    self.tiles.retain(|t| t.id != id);
                    self.ui.expanded_slots.clear();
                    self.status_message = "Tile removed from canvas.".to_string();
                }
                Task::none()
            }
            Message::MacroKeySelected(key) => {
                self.ui.selected_macro_key = key;
                Task::none()
            }
            Message::MacroDelayChanged(val) => {
                self.ui.macro_delay_input = val;
                Task::none()
            }
            Message::MediaActionSelected(action) => {
                self.ui.selected_media_action = action;
                Task::none()
            }
            Message::MacroTextChanged(text_val) => {
                self.ui.macro_text_input = text_val;
                Task::none()
            }
            Message::PresetSelected(preset_name) => {
                self.ui.selected_preset = preset_name;
                Task::none()
            }
            Message::AdvancedModeToggled(val) => {
                self.ui.advanced_mode = val;
                save_user_settings(&UserSettings {
                    last_profile: Some(self.current_profile_name.clone()),
                    advanced_mode: self.ui.advanced_mode,
                });
                Task::none()
            }
            Message::RawMacroStringChanged(slot_key, raw) => {
                if let Some(id) = &self.selected_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                    && let Some(func) = tile.functions.get_mut(&slot_key)
                {
                    func.macro_str = raw;
                }
                Task::none()
            }
            Message::AddMacroAction(slot_key, action) => {
                if let Some(id) = &self.selected_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                    && let Some(func) = tile.functions.get_mut(&slot_key)
                {
                    let mut actions = parse_macro(&func.macro_str);
                    actions.push(action);
                    func.macro_str = compile_macro(&actions);
                }
                Task::none()
            }
            Message::RemoveMacroAction(slot_key, index) => {
                if let Some(id) = &self.selected_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                    && let Some(func) = tile.functions.get_mut(&slot_key)
                {
                    let mut actions = parse_macro(&func.macro_str);
                    if index < actions.len() {
                        actions.remove(index);
                        func.macro_str = compile_macro(&actions);
                    }
                }
                Task::none()
            }
            Message::MoveMacroActionUp(slot_key, index) => {
                if index > 0
                    && let Some(id) = &self.selected_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                    && let Some(func) = tile.functions.get_mut(&slot_key)
                {
                    let mut actions = parse_macro(&func.macro_str);
                    if index < actions.len() {
                        actions.swap(index, index - 1);
                        func.macro_str = compile_macro(&actions);
                    }
                }
                Task::none()
            }
            Message::MoveMacroActionDown(slot_key, index) => {
                if let Some(id) = &self.selected_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                    && let Some(func) = tile.functions.get_mut(&slot_key)
                {
                    let mut actions = parse_macro(&func.macro_str);
                    if index + 1 < actions.len() {
                        actions.swap(index, index + 1);
                        func.macro_str = compile_macro(&actions);
                    }
                }
                Task::none()
            }
            Message::ApplyPreset(slot_key, macro_str) => {
                if let Some(id) = &self.selected_tile_id
                    && let Some(tile) = self.tiles.iter_mut().find(|t| t.id == *id)
                    && let Some(func) = tile.functions.get_mut(&slot_key)
                {
                    func.macro_str = macro_str;
                }
                Task::none()
            }
        }
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
        if let Some(port) = &mut self.serial_port {
            match fetch_config_from_device(port) {
                Ok(json_str) => {
                    if let Ok(cfg) = serde_json::from_str::<FlexPadConfig>(&json_str) {
                        self.tiles = cfg.tiles;
                        let profile_name = self
                            .connected_device
                            .as_ref()
                            .map(|dev| dev.device_id.clone())
                            .unwrap_or_else(|| "FlexPad_Config".to_string());

                        self.current_profile_name = profile_name.clone();
                        self.ui.pending_overwrite = false;

                        let profile_to_save = FlexPadConfig {
                            profile_name: profile_name.clone(),
                            tiles: self.tiles.clone(),
                        };

                        if save_profile_to_disk(&profile_to_save).is_ok() {
                            let raw_profiles = list_saved_profiles();
                            self.saved_profiles = raw_profiles
                                .into_iter()
                                .map(|(name, p)| ProfileItem { name, path: p })
                                .collect();

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
                        self.status_message =
                            "Configuration loaded from device and saved locally.".to_string();
                    } else {
                        self.status_message = "Failed to parse JSON.".to_string();
                    }
                }
                Err(e) => {
                    self.serial_port = None;
                    self.connected_device = None;
                    self.status_message = format!("Error: {}. Disconnected.", e);
                }
            }
        }
    }

    fn handle_set_config(&mut self) {
        if let Some(port) = &mut self.serial_port {
            let config = FlexPadConfig {
                profile_name: self.current_profile_name.clone(),
                tiles: self.tiles.clone(),
            };

            if let Ok(json_str) = serde_json::to_string(&config) {
                match set_config_to_device(port, &json_str) {
                    Ok(_) => {
                        self.status_message =
                            "Configuration saved to device successfully!".to_string();
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save config: {}", e);
                    }
                }
            }
        } else {
            self.status_message = "Not connected to any device.".to_string();
        }
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

    fn handle_profile_selected(&mut self, profile: ProfileItem) {
        if let Ok(cfg) = load_profile_from_disk(&profile.path) {
            self.tiles = cfg.tiles;
            self.current_profile_name = cfg.profile_name.clone();
            self.selected_profile = Some(profile);
            self.ui.pending_overwrite = false;
            self.selected_tile_id = None;
            self.ui.expanded_slots.clear();

            save_user_settings(&UserSettings {
                last_profile: Some(cfg.profile_name),
                advanced_mode: self.ui.advanced_mode,
            });
            self.status_message = "Loaded profile from disk.".to_string();
        }
    }

    fn handle_save_profile(&mut self) -> Task<Message> {
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
            return Task::none();
        }

        self.ui.pending_overwrite = false;

        let config = FlexPadConfig {
            profile_name: self.current_profile_name.clone(),
            tiles: self.tiles.clone(),
        };

        if let Some(selected) = &self.selected_profile
            && selected.name != self.current_profile_name
            && selected.path.exists()
        {
            let _ = std::fs::remove_file(&selected.path);
        }

        match save_profile_to_disk(&config) {
            Ok(_) => {
                let raw_profiles = list_saved_profiles();
                self.saved_profiles = raw_profiles
                    .into_iter()
                    .map(|(name, p)| ProfileItem { name, path: p })
                    .collect();
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
        Task::none()
    }

    fn handle_new_profile(&mut self) {
        let new_name = format!("{}_Copy", self.current_profile_name);
        self.current_profile_name = new_name.clone();
        self.ui.pending_overwrite = false;

        let config = FlexPadConfig {
            profile_name: new_name.clone(),
            tiles: self.tiles.clone(),
        };
        if save_profile_to_disk(&config).is_ok() {
            let raw_profiles = list_saved_profiles();
            self.saved_profiles = raw_profiles
                .into_iter()
                .map(|(name, p)| ProfileItem { name, path: p })
                .collect();
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
                self.ui.expanded_slots.clear();
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
        let tile_type = self.selected_tile_type.unwrap_or(TileType::Button);
        let mut functions_map = BTreeMap::new();

        match tile_type {
            TileType::Button => {
                functions_map.insert(
                    "button".to_string(),
                    TileFunction {
                        pin: "".to_string(),
                        macro_str: "P:lctrl,P:c,D:50,R:c,R:lctrl".to_string(),
                    },
                );
            }
            TileType::EC11 => {
                for slot in ["rot_a", "rot_b", "push"] {
                    functions_map.insert(
                        slot.to_string(),
                        TileFunction {
                            pin: "".to_string(),
                            macro_str: "".to_string(),
                        },
                    );
                }
            }
        }

        let new_id = format!("tile_{}", uuid_short());
        let new_tile = CanvasTile {
            id: new_id.clone(),
            display_name: format!("{} {}", tile_type, self.tiles.len() + 1),
            tile_type,
            functions: functions_map,
            x: 50.0,
            y: 50.0,
        };

        self.tiles.push(new_tile);
        self.selected_tile_id = Some(new_id);
        self.ui.expanded_slots.clear();
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

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:x}", since_epoch.subsec_nanos())
}

use crate::app::{FlexPadApp, Message};
use crate::config::{MacroAction, TileType, parse_macro};
use crate::constants::{AVAILABLE_KEYS, FIRMWARE_PINS, MEDIA_ACTIONS, PRESETS};
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};

pub fn view_inspector<'a>(app: &'a FlexPadApp) -> Element<'a, Message> {
    if let Some(selected_id) = &app.selected_tile_id
        && let Some(tile) = app.tiles.iter().find(|t| t.id == *selected_id)
    {
        let advanced_toggler = checkbox(app.ui.advanced_mode)
            .label("Advanced View")
            .on_toggle(Message::AdvancedModeToggled);

        let mut inspector_col = column![
            row![
                text("Tile Inspector").size(18),
                container(advanced_toggler).align_x(iced::alignment::Horizontal::Right)
            ]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center),
            text(format!("Type: {}", tile.tile_type)).size(14),
            column![
                text("Display Name:").size(12),
                text_input("Name", &tile.display_name)
                    .on_input(Message::UpdateTileName)
                    .padding(5),
            ]
            .spacing(5),
        ]
        .spacing(12);

        let slots_to_render = match tile.tile_type {
            TileType::Button => vec![("button", "Button Pin")],
            TileType::EC11 => vec![
                ("rot_a", "Rotation Pin A"),
                ("rot_b", "Rotation Pin B"),
                ("push", "Push Button Pin"),
            ],
        };

        for (slot_key, section_label) in slots_to_render {
            if let Some(func) = tile.functions.get(slot_key) {
                let pin_num = &func.pin;
                let macro_val = &func.macro_str;
                let is_expanded = app.ui.expanded_slots.contains(&slot_key.to_string());
                let toggle_arrow = if is_expanded { "▼" } else { "▶" };

                let pin_display = if pin_num.is_empty() {
                    "[Unassigned]".to_string()
                } else {
                    format!("[GPIO {}]", pin_num)
                };

                let header_text = format!("{} {} {}", toggle_arrow, section_label, pin_display);

                let toggle_btn = button(text(header_text).size(12))
                    .on_press(Message::ToggleSlotSection(slot_key.to_string()))
                    .width(Length::Fill);

                let mut pin_section = column![toggle_btn].spacing(6);

                if is_expanded {
                    let assigned_pins: Vec<String> = app
                        .tiles
                        .iter()
                        .filter(|t| t.id != *selected_id)
                        .flat_map(|t| t.functions.values().map(|f| f.pin.clone()))
                        .filter(|p| !p.is_empty())
                        .collect();

                    let current_tile_pins: Vec<String> =
                        tile.functions.values().map(|f| f.pin.clone()).collect();

                    let mut pin_options: Vec<String> = vec!["-".to_string()];
                    for p in FIRMWARE_PINS {
                        let p_str = p.to_string();
                        if !assigned_pins.contains(&p_str) || current_tile_pins.contains(&p_str) {
                            pin_options.push(p_str);
                        }
                    }

                    if !pin_num.is_empty() && !pin_options.contains(pin_num) {
                        pin_options.push(pin_num.clone());
                        pin_options.sort_by(|a, b| {
                            if a == "-" {
                                return std::cmp::Ordering::Less;
                            }
                            if b == "-" {
                                return std::cmp::Ordering::Greater;
                            }
                            a.parse::<i32>()
                                .unwrap_or(0)
                                .cmp(&b.parse::<i32>().unwrap_or(0))
                        });
                    }

                    let selected_pin_str = if pin_num.is_empty() {
                        "-".to_string()
                    } else {
                        pin_num.clone()
                    };

                    let pin_picker = pick_list(pin_options, Some(selected_pin_str), {
                        let s_key = slot_key.to_string();
                        move |p_new| {
                            let resolved_pin = if p_new == "-" { "".to_string() } else { p_new };
                            Message::UpdateTilePin(s_key.clone(), resolved_pin)
                        }
                    });

                    let actions = parse_macro(macro_val);
                    let mut macro_list = column![text("Macro Sequence:").size(11)].spacing(4);

                    if actions.is_empty() {
                        macro_list = macro_list.push(text("(No actions)").size(10));
                    } else {
                        let total_actions = actions.len();
                        for (a_idx, action) in actions.into_iter().enumerate() {
                            let action_label = text(format!("{}. {}", a_idx + 1, action)).size(11);
                            let s_target = slot_key.to_string();
                            let remove_btn =
                                button("X").on_press(Message::RemoveMacroAction(s_target, a_idx));

                            let mut item_row = row![action_label]
                                .spacing(6)
                                .align_y(iced::Alignment::Center);

                            if total_actions > 1 {
                                if a_idx > 0 {
                                    let s_up = slot_key.to_string();
                                    let up_btn = button("↑")
                                        .on_press(Message::MoveMacroActionUp(s_up, a_idx));
                                    item_row = item_row.push(up_btn);
                                }

                                if a_idx + 1 < total_actions {
                                    let s_down = slot_key.to_string();
                                    let down_btn = button("↓")
                                        .on_press(Message::MoveMacroActionDown(s_down, a_idx));
                                    item_row = item_row.push(down_btn);
                                }
                            }

                            item_row = item_row.push(remove_btn);
                            macro_list = macro_list.push(item_row);
                        }
                    }

                    let key_labels: Vec<String> =
                        AVAILABLE_KEYS.iter().map(|k| k.0.to_string()).collect();
                    let selected_key_label = AVAILABLE_KEYS
                        .iter()
                        .find(|k| k.1 == app.ui.selected_macro_key)
                        .map(|k| k.0.to_string())
                        .unwrap_or_else(|| AVAILABLE_KEYS[0].0.to_string());

                    let key_picker = pick_list(key_labels, Some(selected_key_label), {
                        let keys_map = AVAILABLE_KEYS.to_vec();
                        move |lbl| {
                            if let Some(found) = keys_map.iter().find(|k| k.0 == lbl) {
                                Message::MacroKeySelected(found.1.to_string())
                            } else {
                                Message::MacroKeySelected("lctrl".to_string())
                            }
                        }
                    });

                    let s_press = slot_key.to_string();
                    let press_btn = button("Press").on_press(Message::AddMacroAction(
                        s_press,
                        MacroAction::PressKey(app.ui.selected_macro_key.clone()),
                    ));
                    let s_release = slot_key.to_string();
                    let release_btn = button("Release").on_press(Message::AddMacroAction(
                        s_release,
                        MacroAction::ReleaseKey(app.ui.selected_macro_key.clone()),
                    ));
                    let key_row = row![key_picker, press_btn, release_btn].spacing(4);

                    let delay_input = text_input("ms", &app.ui.macro_delay_input)
                        .on_input(Message::MacroDelayChanged)
                        .width(60)
                        .padding(4);
                    let s_delay = slot_key.to_string();
                    let add_delay_btn = button("Delay").on_press(Message::AddMacroAction(
                        s_delay,
                        MacroAction::Delay(app.ui.macro_delay_input.parse().unwrap_or(50)),
                    ));
                    let delay_row = row![delay_input, add_delay_btn].spacing(4);

                    let media_labels: Vec<String> =
                        MEDIA_ACTIONS.iter().map(|m| m.0.to_string()).collect();
                    let selected_media_label = MEDIA_ACTIONS
                        .iter()
                        .find(|m| m.1 == app.ui.selected_media_action)
                        .map(|m| m.0.to_string())
                        .unwrap_or_else(|| MEDIA_ACTIONS[0].0.to_string());

                    let media_picker = pick_list(media_labels, Some(selected_media_label), {
                        let media_map = MEDIA_ACTIONS.to_vec();
                        move |lbl| {
                            if let Some(found) = media_map.iter().find(|m| m.0 == lbl) {
                                Message::MediaActionSelected(found.1.to_string())
                            } else {
                                Message::MediaActionSelected("vol_up".to_string())
                            }
                        }
                    });
                    let s_media = slot_key.to_string();
                    let add_media_btn = button("Media").on_press(Message::AddMacroAction(
                        s_media,
                        MacroAction::Media(app.ui.selected_media_action.clone()),
                    ));
                    let media_row = row![media_picker, add_media_btn].spacing(4);

                    let text_input_field = text_input("Text...", &app.ui.macro_text_input)
                        .on_input(Message::MacroTextChanged)
                        .padding(4);
                    let s_text = slot_key.to_string();
                    let add_text_btn = button("Text").on_press(Message::AddMacroAction(
                        s_text,
                        MacroAction::TypeText(app.ui.macro_text_input.clone()),
                    ));
                    let text_row = row![text_input_field, add_text_btn].spacing(4);

                    let preset_names: Vec<String> =
                        PRESETS.iter().map(|p| p.0.to_string()).collect();
                    let preset_picker = pick_list(
                        preset_names,
                        Some(app.ui.selected_preset.clone()),
                        Message::PresetSelected,
                    );

                    let s_preset = slot_key.to_string();
                    let cur_preset = app.ui.selected_preset.clone();
                    let all_presets = PRESETS.to_vec();
                    let apply_preset_btn = button("Apply Preset").on_press_with(move || {
                        if let Some(found) = all_presets.iter().find(|p| p.0 == cur_preset) {
                            Message::ApplyPreset(s_preset.clone(), found.1.to_string())
                        } else {
                            Message::ApplyPreset(s_preset.clone(), "".to_string())
                        }
                    });

                    let preset_row = row![preset_picker, apply_preset_btn].spacing(4);

                    let mut expanded_content = column![
                        text(section_label).size(12),
                        row![text("GPIO Pin:").size(11), pin_picker]
                            .spacing(8)
                            .align_y(iced::Alignment::Center),
                        macro_list,
                        column![
                            text("Add Action:").size(11),
                            key_row,
                            delay_row,
                            media_row,
                            text_row,
                        ]
                        .spacing(4),
                        column![text("Presets:").size(11), preset_row,].spacing(4),
                    ]
                    .spacing(6);

                    if app.ui.advanced_mode {
                        let s_raw = slot_key.to_string();
                        let raw_input = text_input("Raw Macro String", macro_val)
                            .on_input(move |raw| Message::RawMacroStringChanged(s_raw.clone(), raw))
                            .padding(4);
                        expanded_content = expanded_content.push(
                            column![text("Raw String Editor:").size(11), raw_input].spacing(4),
                        );
                    }

                    pin_section = pin_section.push(container(expanded_content).padding(8));
                }

                inspector_col = inspector_col.push(pin_section);
            }
        }

        inspector_col =
            inspector_col.push(button("Delete Tile").on_press(Message::DeleteSelectedTile));
        return inspector_col.into();
    }

    let type_picker = pick_list(
        TileType::ALL.to_vec(),
        app.selected_tile_type,
        Message::TileTypeSelected,
    )
    .placeholder("Select type...");

    let add_tile_btn = button("Add Tile").on_press(Message::AddTile);

    let add_tile_section = column![
        text("Add New Tile").size(16),
        row![type_picker, add_tile_btn].spacing(10),
    ]
    .spacing(8);

    let mut tile_picker_list = column![
        add_tile_section,
        text("Canvas Inspector").size(18),
        text("Select a tile:").size(14)
    ]
    .spacing(12);
    for tile in &app.tiles {
        let t_id = tile.id.clone();
        let pin_summary = tile
            .functions
            .values()
            .map(|f| {
                if f.pin.is_empty() {
                    "-".to_string()
                } else {
                    f.pin.clone()
                }
            })
            .collect::<Vec<String>>()
            .join(", ");
        let btn = button(text(format!(
            "{} (Pins: {})",
            tile.display_name, pin_summary
        )))
        .on_press(Message::SelectTile(t_id));
        tile_picker_list = tile_picker_list.push(btn);
    }

    tile_picker_list.into()
}

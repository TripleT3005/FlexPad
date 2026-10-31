use crate::app::{FlexPadApp, Message};
use crate::components::{self, Component, Function, Parameter, ParameterKind, PinRole};
use crate::config::{MacroAction, parse_macro};
use crate::constants::{FIRMWARE_PINS, MEDIA_ACTIONS, PRESETS};
use crate::keys::{self, KEYS, KeyboardLayout};
use crate::profile::Tile;
use iced::widget::{Column, button, checkbox, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};

// Constants

const UNASSIGNED: &str = "-";
const DEBOUNCE_CHOICES: &[u32] = &[0, 5, 10, 15, 20, 30, 50, 100, 150, 200];

// Inspector

pub fn view_inspector(app: &FlexPadApp) -> Element<'_, Message> {
    match app.selected_tile() {
        Some(tile) => view_tile(app, tile),
        None => view_tile_list(app),
    }
}

fn view_tile<'a>(app: &'a FlexPadApp, tile: &'a Tile) -> Element<'a, Message> {
    let advanced_toggler = checkbox(app.ui.advanced_mode)
        .label("Advanced View")
        .on_toggle(Message::AdvancedModeToggled);

    let component_name = tile.component().map_or_else(
        || format!("Unknown ({})", tile.component),
        |c| c.name.to_string(),
    );

    let mut inspector_col = column![
        row![
            text("Tile Inspector").size(18),
            container(advanced_toggler).align_x(iced::alignment::Horizontal::Right)
        ]
        .width(Length::Fill)
        .align_y(iced::Alignment::Center),
        text(format!("Type: {}", component_name)).size(14),
        column![
            text("Display Name:").size(12),
            text_input("Name", &tile.display_name)
                .on_input(Message::UpdateTileName)
                .padding(5),
        ]
        .spacing(5),
    ]
    .spacing(12);

    if let Some(component) = tile.component() {
        inspector_col = inspector_col
            .push(view_pins(app, tile, component))
            .push(view_parameters(tile, component));
        for function in component.functions {
            inspector_col = inspector_col.push(view_function(app, tile, function));
        }
    }

    inspector_col
        .push(button("Delete Tile").on_press(Message::DeleteSelectedTile))
        .into()
}

// Pins

fn view_pins<'a>(
    app: &'a FlexPadApp,
    tile: &'a Tile,
    component: &Component,
) -> Element<'a, Message> {
    let mut section = column![text("Pins").size(14)].spacing(6);
    for role in component.pins {
        section = section.push(view_pin_role(app, tile, role));
    }
    section.into()
}

fn view_pin_role<'a>(
    app: &'a FlexPadApp,
    tile: &'a Tile,
    role: &'static PinRole,
) -> Element<'a, Message> {
    let current = tile.pin(role.id);
    let used_elsewhere: Vec<u8> = app
        .tiles
        .iter()
        .flat_map(|other| {
            other
                .pins
                .iter()
                .filter(move |(other_role, _)| {
                    other.id != tile.id || other_role.as_str() != role.id
                })
                .map(|(_, pin)| *pin)
        })
        .collect();

    let mut options = vec![UNASSIGNED.to_string()];
    options.extend(
        FIRMWARE_PINS
            .iter()
            .filter(|pin| !used_elsewhere.contains(pin) || current == Some(**pin))
            .map(u8::to_string),
    );
    if let Some(pin) = current
        && !FIRMWARE_PINS.contains(&pin)
    {
        options.push(pin.to_string());
    }

    let selected = current.map_or_else(|| UNASSIGNED.to_string(), |pin| pin.to_string());
    let picker = pick_list(options, Some(selected), move |choice| {
        Message::UpdateTilePin(role.id.to_string(), choice.parse().ok())
    });

    row![
        text(role.label).size(12).width(Length::Fixed(130.0)),
        picker
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

// Parameters

fn view_parameters<'a>(tile: &'a Tile, component: &Component) -> Element<'a, Message> {
    if component.parameters.is_empty() {
        return column![].into();
    }
    let mut section = column![text("Parameters").size(14)].spacing(6);
    for parameter in component.parameters {
        section = section.push(view_parameter(tile, parameter));
    }
    section.into()
}

fn view_parameter<'a>(tile: &'a Tile, parameter: &'static Parameter) -> Element<'a, Message> {
    let value = tile
        .parameter(parameter.id)
        .unwrap_or_else(|| parameter.default_value());
    let label = text(parameter.label).size(12).width(Length::Fixed(130.0));

    let control: Element<'a, Message> = match parameter.kind {
        ParameterKind::Flag { .. } => checkbox(value.as_bool().unwrap_or(false))
            .on_toggle(move |checked| {
                Message::UpdateParameter(parameter.id.to_string(), checked.into())
            })
            .into(),
        ParameterKind::Number { min, max, .. } => {
            let current = value.as_u64().and_then(|n| u32::try_from(n).ok());
            let mut options: Vec<u32> = DEBOUNCE_CHOICES
                .iter()
                .copied()
                .filter(|choice| (min..=max).contains(choice))
                .collect();
            if let Some(current) = current
                && !options.contains(&current)
            {
                options.push(current);
                options.sort_unstable();
            }
            number_picker(parameter, options, current)
        }
        ParameterKind::Choice { options, .. } => {
            let current = value.as_u64().and_then(|n| u32::try_from(n).ok());
            number_picker(parameter, options.to_vec(), current)
        }
    };

    row![label, control]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
}

fn number_picker<'a>(
    parameter: &'static Parameter,
    options: Vec<u32>,
    current: Option<u32>,
) -> Element<'a, Message> {
    pick_list(options, current, move |choice| {
        Message::UpdateParameter(parameter.id.to_string(), choice.into())
    })
    .into()
}

// Functions

fn view_function<'a>(
    app: &'a FlexPadApp,
    tile: &'a Tile,
    function: &'static Function,
) -> Element<'a, Message> {
    let macro_val = tile.macro_for(function.id);
    let is_expanded = app.ui.expanded_functions.iter().any(|id| id == function.id);
    let toggle_arrow = if is_expanded { "▼" } else { "▶" };
    let summary = if macro_val.is_empty() {
        "[No macro]".to_string()
    } else {
        format!("[{} actions]", parse_macro(macro_val).len())
    };

    let toggle_btn =
        button(text(format!("{} {} {}", toggle_arrow, function.label, summary)).size(12))
            .on_press(Message::ToggleFunctionSection(function.id.to_string()))
            .width(Length::Fill);

    let mut section = column![toggle_btn].spacing(6);
    if is_expanded {
        section =
            section.push(container(view_macro_editor(app, function.id, macro_val)).padding(8));
    }
    section.into()
}

fn view_macro_editor<'a>(
    app: &'a FlexPadApp,
    function_id: &'static str,
    macro_val: &'a str,
) -> Column<'a, Message> {
    let target = || function_id.to_string();

    let actions = parse_macro(macro_val);
    let mut macro_list = column![text("Macro Sequence:").size(11)].spacing(4);
    if actions.is_empty() {
        macro_list = macro_list.push(text("(No actions)").size(10));
    } else {
        let total_actions = actions.len();
        for (a_idx, action) in actions.into_iter().enumerate() {
            let label = action_label(&action, app.keyboard_layout);
            let mut item_row = row![text(format!("{}. {}", a_idx + 1, label)).size(11)]
                .spacing(6)
                .align_y(iced::Alignment::Center);
            if a_idx > 0 {
                item_row = item_row
                    .push(button("↑").on_press(Message::MoveMacroActionUp(target(), a_idx)));
            }
            if a_idx + 1 < total_actions {
                item_row = item_row
                    .push(button("↓").on_press(Message::MoveMacroActionDown(target(), a_idx)));
            }
            item_row =
                item_row.push(button("X").on_press(Message::RemoveMacroAction(target(), a_idx)));
            macro_list = macro_list.push(item_row);
        }
    }

    let layout = app.keyboard_layout;
    let key_labels: Vec<String> = KEYS.iter().map(|k| k.label(layout).to_string()).collect();
    let selected_key_label = keys::label_for(&app.ui.selected_macro_key, layout);
    let key_picker = pick_list(key_labels, Some(selected_key_label), move |lbl| {
        let code = KEYS
            .iter()
            .find(|k| k.label(layout) == lbl)
            .map_or("ControlLeft", |k| k.code);
        Message::MacroKeySelected(code.to_string())
    });
    let press_btn = button("Press").on_press(Message::AddMacroAction(
        target(),
        MacroAction::PressKey(app.ui.selected_macro_key.clone()),
    ));
    let release_btn = button("Release").on_press(Message::AddMacroAction(
        target(),
        MacroAction::ReleaseKey(app.ui.selected_macro_key.clone()),
    ));
    let key_row = row![key_picker, press_btn, release_btn].spacing(4);

    let delay_input = text_input("ms", &app.ui.macro_delay_input)
        .on_input(Message::MacroDelayChanged)
        .width(60)
        .padding(4);
    let add_delay_btn = button("Delay").on_press(Message::AddMacroAction(
        target(),
        MacroAction::Delay(app.ui.macro_delay_input.parse().unwrap_or(50)),
    ));
    let delay_row = row![delay_input, add_delay_btn].spacing(4);

    let media_labels: Vec<String> = MEDIA_ACTIONS.iter().map(|m| m.0.to_string()).collect();
    let selected_media_label = MEDIA_ACTIONS
        .iter()
        .find(|m| m.1 == app.ui.selected_media_action)
        .map(|m| m.0.to_string())
        .unwrap_or_else(|| MEDIA_ACTIONS[0].0.to_string());
    let media_picker = pick_list(media_labels, Some(selected_media_label), |lbl| {
        let action = MEDIA_ACTIONS
            .iter()
            .find(|m| m.0 == lbl)
            .map_or("vol_up", |m| m.1);
        Message::MediaActionSelected(action.to_string())
    });
    let add_media_btn = button("Media").on_press(Message::AddMacroAction(
        target(),
        MacroAction::Media(app.ui.selected_media_action.clone()),
    ));
    let media_row = row![media_picker, add_media_btn].spacing(4);

    let text_input_field = text_input("Text...", &app.ui.macro_text_input)
        .on_input(Message::MacroTextChanged)
        .padding(4);
    let add_text_btn = button("Text").on_press(Message::AddMacroAction(
        target(),
        MacroAction::TypeText(app.ui.macro_text_input.clone()),
    ));
    let text_row = row![text_input_field, add_text_btn].spacing(4);

    let preset_names: Vec<String> = PRESETS.iter().map(|p| p.0.to_string()).collect();
    let preset_picker = pick_list(
        preset_names,
        Some(app.ui.selected_preset.clone()),
        Message::PresetSelected,
    );
    let preset_macro = PRESETS
        .iter()
        .find(|p| p.0 == app.ui.selected_preset)
        .map_or("", |p| p.1);
    let apply_preset_btn =
        button("Apply Preset").on_press(Message::ApplyPreset(target(), preset_macro.to_string()));
    let preset_row = row![preset_picker, apply_preset_btn].spacing(4);

    let mut editor = column![
        macro_list,
        column![
            text("Add Action:").size(11),
            key_row,
            delay_row,
            media_row,
            text_row,
        ]
        .spacing(4),
        column![text("Presets:").size(11), preset_row].spacing(4),
    ]
    .spacing(6);

    if app.ui.advanced_mode {
        let raw_input = text_input("Raw Macro String", macro_val)
            .on_input(move |raw| Message::RawMacroStringChanged(function_id.to_string(), raw))
            .padding(4);
        editor = editor.push(column![text("Raw String Editor:").size(11), raw_input].spacing(4));
    }
    editor
}

// Tile list

fn view_tile_list(app: &FlexPadApp) -> Element<'_, Message> {
    let component_picker = pick_list(
        components::ALL.to_vec(),
        app.selected_component,
        Message::ComponentSelected,
    )
    .placeholder("Select type...");

    let add_tile_btn = button("Add Tile").on_press(Message::AddTile);

    let add_tile_section = column![
        text("Add New Tile").size(16),
        row![component_picker, add_tile_btn].spacing(10),
    ]
    .spacing(8);

    let mut tile_picker_list = column![
        add_tile_section,
        text("Canvas Inspector").size(18),
        text("Select a tile:").size(14)
    ]
    .spacing(12);
    for tile in &app.tiles {
        let btn = button(text(format!(
            "{} (Pins: {})",
            tile.display_name,
            pin_summary(tile)
        )))
        .on_press(Message::SelectTile(tile.id.clone()));
        tile_picker_list = tile_picker_list.push(btn);
    }

    tile_picker_list.into()
}

pub fn pin_summary(tile: &Tile) -> String {
    let roles = tile.component().map_or(&[][..], |component| component.pins);
    let pins: Vec<String> = roles
        .iter()
        .map(|role| {
            tile.pin(role.id)
                .map_or_else(|| UNASSIGNED.to_string(), |pin| pin.to_string())
        })
        .collect();
    if pins.is_empty() {
        UNASSIGNED.to_string()
    } else {
        pins.join(", ")
    }
}

fn action_label(action: &MacroAction, layout: KeyboardLayout) -> String {
    match action {
        MacroAction::PressKey(key) => format!("Press: {}", keys::label_for(key, layout)),
        MacroAction::ReleaseKey(key) => format!("Release: {}", keys::label_for(key, layout)),
        other => other.to_string(),
    }
}

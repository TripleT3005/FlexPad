use crate::app::{FlexPadApp, Message};
use crate::components::{self, Component, Function, Parameter, ParameterKind, PinRole};
use crate::constants::{FIRMWARE_PINS, MEDIA_ACTIONS, PRESETS};
use crate::i18n::{format_text, text as tr};
use crate::keys::{self, KEYS, KeyboardLayout};
use crate::macros::{Macro, MacroAction};
use crate::profile::Tile;
use crate::texts;
use crate::ui::choice::Choice;
use crate::ui::label_button;
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
        .label(tr(texts::INSPECTOR_ADVANCED))
        .on_toggle(Message::AdvancedModeToggled);

    let component_name = tile.component().map_or_else(
        || format_text(texts::INSPECTOR_UNKNOWN_TYPE, &[("type", &tile.component)]),
        ToString::to_string,
    );

    let mut inspector_col = column![
        row![
            text(tr(texts::INSPECTOR_TITLE)).size(18),
            container(advanced_toggler)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
        ]
        .spacing(12)
        .width(Length::Fill)
        .align_y(iced::Alignment::Center),
        text(format_text(
            texts::INSPECTOR_TYPE,
            &[("type", &component_name)]
        ))
        .size(14),
        column![
            text(tr(texts::INSPECTOR_DISPLAY_NAME)).size(12),
            text_input(&tr(texts::INSPECTOR_NAME), &tile.display_name)
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
        if !component.functions.is_empty() {
            let note = format_text(
                texts::INSPECTOR_TEST_NOTE,
                &[("button", &tr(texts::INSPECTOR_TEST))],
            );
            inspector_col = inspector_col.push(
                column![
                    text(tr(texts::INSPECTOR_FUNCTIONS)).size(14),
                    text(note).size(10)
                ]
                .spacing(4),
            );
        }
        for function in component.functions {
            inspector_col = inspector_col.push(view_function(app, tile, function));
        }
    }

    inspector_col
        .push(label_button(tr(texts::INSPECTOR_DELETE)).on_press(Message::DeleteSelectedTile))
        .into()
}

// Pins

fn view_pins<'a>(
    app: &'a FlexPadApp,
    tile: &'a Tile,
    component: &Component,
) -> Element<'a, Message> {
    let mut section = column![text(tr(texts::INSPECTOR_PINS)).size(14)].spacing(6);
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
        text(tr(role.label)).size(12).width(Length::Fixed(130.0)),
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
    let mut section = column![text(tr(texts::INSPECTOR_PARAMETERS)).size(14)].spacing(6);
    for parameter in component.parameters {
        section = section.push(view_parameter(tile, parameter));
    }
    section.into()
}

fn view_parameter<'a>(tile: &'a Tile, parameter: &'static Parameter) -> Element<'a, Message> {
    let value = tile
        .parameter(parameter.id)
        .unwrap_or_else(|| parameter.default_value());
    let label = text(tr(parameter.label))
        .size(12)
        .width(Length::Fixed(130.0));

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
    let actions = tile.macro_for(function.id);
    let is_expanded = app.ui.expanded_functions.iter().any(|id| id == function.id);
    let toggle_arrow = if is_expanded { "▾" } else { "▸" };
    let summary = if actions.is_empty() {
        tr(texts::INSPECTOR_NO_MACRO)
    } else {
        format_text(
            texts::INSPECTOR_ACTIONS,
            &[("count", &actions.len().to_string())],
        )
    };

    let toggle_btn = button(
        text(format!(
            "{} {} {}",
            toggle_arrow,
            tr(function.label),
            summary
        ))
        .size(12),
    )
    .on_press(Message::ToggleFunctionSection(function.id.to_string()))
    .width(Length::Fill);
    let can_test = app.connected_device.is_some() && !app.is_device_busy() && !actions.is_empty();
    let test_btn = button(text(tr(texts::INSPECTOR_TEST)).size(12))
        .on_press_maybe(can_test.then(|| Message::TestMacro(function.id.to_string())));

    let mut section = column![
        row![toggle_btn, test_btn]
            .spacing(6)
            .align_y(iced::Alignment::Center)
    ]
    .spacing(6);
    if let Some(state) = app.ui.macro_test_for(&tile.id, function.id) {
        section = section.push(text(state.to_string()).size(11));
    }
    if is_expanded {
        section = section.push(container(view_macro_editor(app, function.id, actions)).padding(8));
    }
    section.into()
}

fn view_macro_editor<'a>(
    app: &'a FlexPadApp,
    function_id: &'static str,
    actions: &'a [MacroAction],
) -> Column<'a, Message> {
    let target = || function_id.to_string();

    let mut macro_list = column![text(tr(texts::MACRO_SEQUENCE)).size(11)].spacing(4);
    if actions.is_empty() {
        macro_list = macro_list.push(text(tr(texts::MACRO_NO_ACTIONS)).size(10));
    } else {
        let total_actions = actions.len();
        for (a_idx, action) in actions.iter().enumerate() {
            let label = action_label(action, app.keyboard_layout);
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
            if let MacroAction::Text(value) = action {
                let missing = app.keyboard_layout.untypeable_characters(value);
                if !missing.is_empty() {
                    macro_list = macro_list.push(
                        text(format!(
                            "   {}",
                            format_text(
                                texts::MACRO_UNTYPEABLE,
                                &[
                                    ("layout", &app.keyboard_layout.to_string()),
                                    ("characters", &missing),
                                ],
                            )
                        ))
                        .size(10),
                    );
                }
            }
        }
    }

    let layout = app.keyboard_layout;
    let key_choices: Vec<Choice<&'static str>> = keys::picker_keys(layout)
        .iter()
        .map(|key| Choice::new(key.code, key.label(layout)))
        .collect();
    let selected_key = KEYS
        .iter()
        .find(|key| key.code == app.ui.selected_macro_key)
        .map(|key| Choice::new(key.code, key.label(layout)));
    let key_picker = pick_list(key_choices, selected_key, |choice| {
        Message::MacroKeySelected(choice.value.to_string())
    });
    let press_btn = label_button(tr(texts::MACRO_PRESS)).on_press(Message::AddMacroAction(
        target(),
        MacroAction::Press(app.ui.selected_macro_key.clone()),
    ));
    let release_btn = label_button(tr(texts::MACRO_RELEASE)).on_press(Message::AddMacroAction(
        target(),
        MacroAction::Release(app.ui.selected_macro_key.clone()),
    ));
    let key_row = row![key_picker, press_btn, release_btn].spacing(4);

    let delay_input = text_input(&tr(texts::MACRO_DELAY_UNIT), &app.ui.macro_delay_input)
        .on_input(Message::MacroDelayChanged)
        .width(60)
        .padding(4);
    let add_delay_btn = label_button(tr(texts::MACRO_DELAY)).on_press(Message::AddMacroAction(
        target(),
        MacroAction::Delay(app.ui.macro_delay_input.parse().unwrap_or(50)),
    ));
    let delay_row = row![delay_input, add_delay_btn].spacing(4);

    let media_choices: Vec<Choice<&'static str>> = MEDIA_ACTIONS
        .iter()
        .map(|&(label, action)| Choice::new(action, tr(label)))
        .collect();
    let selected_media = media_choices
        .iter()
        .find(|choice| choice.value == app.ui.selected_media_action)
        .or(media_choices.first())
        .cloned();
    let media_picker = pick_list(media_choices, selected_media, |choice| {
        Message::MediaActionSelected(choice.value.to_string())
    });
    let add_media_btn = label_button(tr(texts::MACRO_MEDIA)).on_press(Message::AddMacroAction(
        target(),
        MacroAction::Media(app.ui.selected_media_action.clone()),
    ));
    let media_row = row![media_picker, add_media_btn].spacing(4);

    let text_input_field = text_input(&tr(texts::MACRO_TEXT_PLACEHOLDER), &app.ui.macro_text_input)
        .on_input(Message::MacroTextChanged)
        .padding(4);
    let add_text_btn = label_button(tr(texts::MACRO_TEXT)).on_press(Message::AddMacroAction(
        target(),
        MacroAction::Text(app.ui.macro_text_input.clone()),
    ));
    let text_row = row![text_input_field, add_text_btn].spacing(4);

    let preset_choices: Vec<Choice<&'static str>> = PRESETS
        .iter()
        .map(|&(name, _)| Choice::new(name, tr(name)))
        .collect();
    let selected_preset = PRESETS
        .iter()
        .find(|p| p.0 == app.ui.selected_preset)
        .or(PRESETS.first());
    let preset_picker = pick_list(
        preset_choices,
        selected_preset.map(|&(name, _)| Choice::new(name, tr(name))),
        |choice| Message::PresetSelected(choice.value.to_string()),
    );
    let mut apply_preset_btn = label_button(tr(texts::MACRO_APPLY_PRESET));
    if let Some(&(_, preset)) = selected_preset {
        apply_preset_btn = apply_preset_btn.on_press(Message::ApplyPreset(target(), preset));
    }
    let preset_row = row![preset_picker, apply_preset_btn].spacing(4);

    let mut editor = column![
        macro_list,
        column![
            text(tr(texts::MACRO_ADD_ACTION)).size(11),
            key_row,
            delay_row,
            media_row,
            text_row,
        ]
        .spacing(4),
        column![text(tr(texts::MACRO_PRESETS)).size(11), preset_row].spacing(4),
    ]
    .spacing(6);

    if app.ui.advanced_mode {
        let pending = app
            .ui
            .raw_macro_input
            .as_ref()
            .filter(|(function, _)| function == function_id)
            .map(|(_, raw)| raw.clone());
        let is_invalid = pending
            .as_ref()
            .is_some_and(|raw| serde_json::from_str::<Macro>(raw).is_err());
        let raw_value = pending.unwrap_or_else(|| {
            serde_json::to_string(&Macro {
                actions: actions.to_vec(),
            })
            .unwrap_or_default()
        });
        let raw_input = text_input("{\"actions\":[]}", &raw_value)
            .on_input(move |raw| Message::RawMacroChanged(function_id.to_string(), raw))
            .padding(4);
        let mut raw_section =
            column![text(tr(texts::MACRO_RAW_JSON)).size(11), raw_input].spacing(4);
        if is_invalid {
            raw_section = raw_section.push(text(tr(texts::MACRO_INVALID_JSON)).size(10));
        }
        editor = editor.push(raw_section);
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
    .placeholder(tr(texts::TILES_SELECT_TYPE))
    .width(Length::Fill);

    let add_tile_btn = label_button(tr(texts::TILES_ADD)).on_press(Message::AddTile);

    let add_tile_section = column![
        text(tr(texts::TILES_ADD_TITLE)).size(16),
        row![component_picker, add_tile_btn].spacing(10),
    ]
    .spacing(8);

    let mut tile_picker_list = column![
        add_tile_section,
        text(tr(texts::TILES_CANVAS_TITLE)).size(18),
        text(tr(texts::TILES_SELECT)).size(14)
    ]
    .spacing(12);
    for tile in &app.tiles {
        let btn = button(text(format_text(
            texts::TILES_ENTRY,
            &[("name", &tile.display_name), ("pins", &pin_summary(tile))],
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
        MacroAction::Press(key) => format_text(
            texts::ACTION_PRESS,
            &[("key", &keys::label_for(key, layout))],
        ),
        MacroAction::Release(key) => format_text(
            texts::ACTION_RELEASE,
            &[("key", &keys::label_for(key, layout))],
        ),
        other => other.to_string(),
    }
}

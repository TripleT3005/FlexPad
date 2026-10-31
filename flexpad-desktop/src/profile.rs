use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::components::{self, BUTTON, Component, EC11};
use crate::constants::FIRMWARE_PINS;
use crate::keys::KeyboardLayout;
use crate::macros::{self, Macro, MacroAction};

// Constants

pub const FORMAT: u32 = 2;
pub const DEFAULT_PROFILE_NAME: &str = "Default_Profile";
const LEGACY_PIN_SPACING: f32 = 120.0;
const LEGACY_TILES_PER_ROW: usize = 5;

// Profile

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub id: String,
    pub display_name: String,
    pub component: String,
    pub kind: String,
    #[serde(default)]
    pub pins: BTreeMap<String, u8>,
    #[serde(default)]
    pub parameters: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub functions: BTreeMap<String, Macro>,
    pub x: f32,
    pub y: f32,
}

impl Tile {
    pub fn new(component: &Component, id: String, display_name: String, x: f32, y: f32) -> Self {
        Self {
            id,
            display_name,
            component: component.id.to_string(),
            kind: component.kind.to_string(),
            pins: BTreeMap::new(),
            parameters: component
                .parameters
                .iter()
                .map(|parameter| (parameter.id.to_string(), parameter.default_value()))
                .collect(),
            functions: component
                .functions
                .iter()
                .map(|function| (function.id.to_string(), function.default_macro.to_macro()))
                .collect(),
            x,
            y,
        }
    }

    pub fn component(&self) -> Option<&'static Component> {
        components::find(&self.component)
    }

    pub fn pin(&self, role: &str) -> Option<u8> {
        self.pins.get(role).copied()
    }

    pub fn set_pin(&mut self, role: &str, pin: Option<u8>) {
        match pin {
            Some(pin) => self.pins.insert(role.to_string(), pin),
            None => self.pins.remove(role),
        };
    }

    pub fn parameter(&self, id: &str) -> Option<serde_json::Value> {
        self.parameters.get(id).cloned().or_else(|| {
            self.component()
                .and_then(|component| component.parameter(id))
                .map(|parameter| parameter.default_value())
        })
    }

    pub fn macro_for(&self, function: &str) -> &[MacroAction] {
        self.functions
            .get(function)
            .map_or(&[], |value| value.actions.as_slice())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub format: u32,
    pub profile_name: String,
    #[serde(default)]
    pub keyboard_layout: KeyboardLayout,
    #[serde(default)]
    pub tiles: Vec<Tile>,
}

impl Profile {
    pub fn new(profile_name: String, tiles: Vec<Tile>) -> Self {
        Self {
            format: FORMAT,
            profile_name,
            keyboard_layout: KeyboardLayout::default(),
            tiles,
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| error.to_string())
    }

    pub fn to_pretty_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self::new(
            DEFAULT_PROFILE_NAME.to_string(),
            vec![Tile::new(
                &BUTTON,
                "tile_1".to_string(),
                "Button 1".to_string(),
                60.0,
                60.0,
            )],
        )
    }
}

// Loading

pub fn parse_profile(json: &str) -> Result<Profile, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| error.to_string())?;
    if !value.is_object() {
        return Err("The profile is not a JSON object.".to_string());
    }
    let mut profile: Profile = match value.get("format") {
        None => migrate_legacy(value)?,
        Some(format) if format.as_u64() == Some(u64::from(FORMAT)) => {
            migrate_macro_strings(&mut value);
            serde_json::from_value(value).map_err(|error| error.to_string())?
        }
        Some(format) => return Err(format!("Unsupported profile format: {format}")),
    };
    for tile in &mut profile.tiles {
        for function in tile.functions.values_mut() {
            function.migrate_keys();
        }
    }
    Ok(profile)
}

fn migrate_macro_strings(value: &mut serde_json::Value) {
    let Some(tiles) = value
        .get_mut("tiles")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let functions = tiles
        .iter_mut()
        .filter_map(|tile| tile.get_mut("functions"))
        .filter_map(serde_json::Value::as_object_mut);
    for function in functions.flat_map(|functions| functions.values_mut()) {
        if let Some(macro_str) = function.as_str() {
            *function = serde_json::to_value(macros::parse_legacy(macro_str))
                .unwrap_or(serde_json::Value::Null);
        }
    }
}

// Legacy formats

#[derive(Deserialize)]
struct LegacyProfile {
    profile_name: Option<String>,
    #[serde(default)]
    tiles: Vec<LegacyTile>,
    #[serde(default)]
    pins: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct LegacyTile {
    id: String,
    display_name: String,
    tile_type: LegacyTileType,
    #[serde(default)]
    functions: BTreeMap<String, LegacyFunction>,
    #[serde(default)]
    pins: BTreeMap<String, String>,
    x: f32,
    y: f32,
}

#[derive(Deserialize)]
enum LegacyTileType {
    Button,
    EC11,
}

#[derive(Deserialize)]
struct LegacyFunction {
    pin: String,
    macro_str: String,
}

const BUTTON_SLOTS: &[(&str, &str, &str)] = &[("button", "switch", "press")];
const EC11_SLOTS: &[(&str, &str, &str)] = &[
    ("rot_a", "a", "turn_right"),
    ("rot_b", "b", "turn_left"),
    ("push", "switch", "press"),
];

fn usable_pin(text: &str) -> Option<u8> {
    text.trim()
        .parse()
        .ok()
        .filter(|pin| FIRMWARE_PINS.contains(pin))
}

fn migrate_legacy(value: serde_json::Value) -> Result<Profile, String> {
    let legacy: LegacyProfile = serde_json::from_value(value).map_err(|error| error.to_string())?;
    let mut tiles: Vec<Tile> = legacy.tiles.into_iter().map(migrate_tile).collect();
    let assigned_pins = legacy
        .pins
        .iter()
        .filter(|(_, macro_text)| !macro_text.trim().is_empty());
    for (index, (pin, macro_text)) in assigned_pins.enumerate() {
        let mut tile = Tile::new(
            &BUTTON,
            format!("tile_gpio_{pin}"),
            format!("GPIO {pin}"),
            60.0 + (index % LEGACY_TILES_PER_ROW) as f32 * LEGACY_PIN_SPACING,
            60.0 + (index / LEGACY_TILES_PER_ROW) as f32 * LEGACY_PIN_SPACING,
        );
        tile.set_pin("switch", usable_pin(pin));
        tile.functions
            .insert("press".to_string(), macros::parse_legacy(macro_text));
        tiles.push(tile);
    }
    Ok(Profile::new(
        legacy
            .profile_name
            .unwrap_or_else(|| DEFAULT_PROFILE_NAME.to_string()),
        tiles,
    ))
}

fn migrate_tile(legacy: LegacyTile) -> Tile {
    let (component, slots) = match legacy.tile_type {
        LegacyTileType::Button => (&BUTTON, BUTTON_SLOTS),
        LegacyTileType::EC11 => (&EC11, EC11_SLOTS),
    };
    let mut tile = Tile::new(
        component,
        legacy.id,
        legacy.display_name,
        legacy.x,
        legacy.y,
    );
    if legacy.functions.is_empty() && !legacy.pins.is_empty() {
        for (&(_, role, function), (pin, macro_text)) in slots.iter().zip(&legacy.pins) {
            tile.set_pin(role, usable_pin(pin));
            tile.functions
                .insert(function.to_string(), macros::parse_legacy(macro_text));
        }
        return tile;
    }
    for &(slot, role, function) in slots {
        let Some(old) = legacy.functions.get(slot) else {
            tile.functions
                .insert(function.to_string(), Macro::default());
            continue;
        };
        tile.set_pin(role, usable_pin(&old.pin));
        tile.functions
            .insert(function.to_string(), macros::parse_legacy(&old.macro_str));
    }
    tile
}

#[cfg(test)]
mod tests {
    use super::*;
    fn legacy(macro_str: &str) -> Vec<MacroAction> {
        macros::parse_legacy(macro_str).actions
    }

    // Format 2

    #[test]
    fn new_tiles_use_component_defaults() {
        let tile = Tile::new(&EC11, "t".to_string(), "Volume".to_string(), 0.0, 0.0);

        assert_eq!(tile.kind, "encoder");
        assert!(tile.pins.is_empty());
        assert_eq!(
            tile.parameter("steps_per_detent"),
            Some(serde_json::json!(4))
        );
        assert_eq!(tile.parameter("reverse"), Some(serde_json::json!(false)));
        assert_eq!(tile.macro_for("turn_right"), legacy("MEDIA:vol_up"));
        assert!(tile.macro_for("unknown").is_empty());
    }

    #[test]
    fn profile_survives_json_roundtrip() {
        let mut profile = Profile::default();
        profile.tiles[0].set_pin("switch", Some(4));

        let json = profile.to_json().unwrap();
        let loaded = parse_profile(&json).unwrap();

        assert_eq!(loaded, profile);
        assert!(json.starts_with(r#"{"format":2,"#));
        assert!(json.contains(r#""pins":{"switch":4}"#));
    }

    #[test]
    fn matches_the_firmware_example() {
        let json = r#"{"format":2,"profile_name":"Desk","keyboard_layout":"us","tiles":[{"id":"tile_2","display_name":"Volume","component":"ec11","kind":"encoder","pins":{"a":15,"b":16,"switch":17},"parameters":{"steps_per_detent":4,"reverse":false,"debounce_ms":10},"functions":{"turn_right":{"actions":[{"media":"vol_up"}]},"turn_left":{"actions":[{"media":"vol_down"}]},"press":{"actions":[{"media":"mute"}]}},"x":200.0,"y":50.0}]}"#;

        let profile = parse_profile(json).unwrap();

        assert_eq!(profile.tiles[0].pin("b"), Some(16));
        assert_eq!(profile.tiles[0].component(), Some(&EC11));
        assert_eq!(profile.tiles[0].macro_for("press"), legacy("MEDIA:mute"));
        assert_eq!(parse_profile(&profile.to_json().unwrap()).unwrap(), profile);
    }

    #[test]
    fn converts_macro_strings_of_format_2() {
        let json = r#"{"format":2,"profile_name":"P","tiles":[{"id":"t","display_name":"T","component":"button","kind":"button","pins":{"switch":4},"functions":{"press":"P:lctrl,TEXT:Hallo, Welt,D:10,R:lctrl"},"x":0.0,"y":0.0}]}"#;

        let profile = parse_profile(json).unwrap();

        assert_eq!(
            profile.tiles[0].macro_for("press"),
            [
                MacroAction::Press("ControlLeft".to_string()),
                MacroAction::Text("Hallo, Welt".to_string()),
                MacroAction::Delay(10),
                MacroAction::Release("ControlLeft".to_string()),
            ]
        );
        assert!(profile.to_json().unwrap().contains(
            r#""functions":{"press":{"actions":[{"press":"ControlLeft"},{"text":"Hallo, Welt"},{"delay":10},{"release":"ControlLeft"}]}}"#
        ));
    }

    #[test]
    fn rejects_invalid_function_objects() {
        let json = r#"{"format":2,"profile_name":"P","tiles":[{"id":"t","display_name":"T","component":"button","kind":"button","functions":{"press":{"actions":[{"jump":1}]}},"x":0.0,"y":0.0}]}"#;

        assert!(parse_profile(json).is_err());
    }

    #[test]
    fn set_pin_none_removes_the_role() {
        let mut tile = Tile::new(&BUTTON, "t".to_string(), "B".to_string(), 0.0, 0.0);
        tile.set_pin("switch", Some(4));
        tile.set_pin("switch", None);

        assert!(tile.pins.is_empty());
    }

    #[test]
    fn rejects_unknown_formats_and_non_objects() {
        assert!(parse_profile(r#"{"format":3}"#).is_err());
        assert!(parse_profile(r#"{"format":"2"}"#).is_err());
        assert!(parse_profile("[]").is_err());
        assert!(parse_profile("{").is_err());
    }

    // Legacy formats

    #[test]
    fn migrates_format_1_profiles() {
        let json = r#"{"profile_name":"Old","tiles":[
            {"id":"t1","display_name":"Copy","tile_type":"Button","functions":{"button":{"pin":"4","macro_str":"P:lctrl,P:c,R:c,R:lctrl"}},"x":10.0,"y":20.0},
            {"id":"t2","display_name":"Volume","tile_type":"EC11","functions":{
                "rot_a":{"pin":"15","macro_str":"MEDIA:vol_up"},
                "rot_b":{"pin":"16","macro_str":"MEDIA:vol_down"},
                "push":{"pin":"","macro_str":"MEDIA:mute"}},"x":200.0,"y":20.0}
        ]}"#;

        let profile = parse_profile(json).unwrap();

        assert_eq!(profile.format, FORMAT);
        assert_eq!(profile.profile_name, "Old");
        let button = &profile.tiles[0];
        assert_eq!(
            (button.component.as_str(), button.kind.as_str()),
            ("button", "button")
        );
        assert_eq!(button.pin("switch"), Some(4));
        assert_eq!(
            button.macro_for("press"),
            legacy("P:ControlLeft,P:KeyC,R:KeyC,R:ControlLeft")
        );
        assert_eq!((button.x, button.y), (10.0, 20.0));
        let encoder = &profile.tiles[1];
        assert_eq!(encoder.kind, "encoder");
        assert_eq!(
            (encoder.pin("a"), encoder.pin("b"), encoder.pin("switch")),
            (Some(15), Some(16), None)
        );
        assert_eq!(encoder.macro_for("turn_right"), legacy("MEDIA:vol_up"));
        assert_eq!(encoder.macro_for("turn_left"), legacy("MEDIA:vol_down"));
        assert_eq!(encoder.macro_for("press"), legacy("MEDIA:mute"));
        assert_eq!(
            encoder.parameter("steps_per_detent"),
            Some(serde_json::json!(4))
        );
    }

    #[test]
    fn migrates_arduino_pins() {
        let profile =
            parse_profile(r#"{"pins":{"15":"MEDIA:mute","5":"P:ctrl,P:c,R:c,R:ctrl"}}"#).unwrap();

        assert_eq!(profile.profile_name, DEFAULT_PROFILE_NAME);
        assert_eq!(profile.tiles.len(), 2);
        assert!(
            profile
                .tiles
                .iter()
                .any(|tile| tile.pin("switch") == Some(15)
                    && tile.macro_for("press") == legacy("MEDIA:mute"))
        );
        assert!(profile.tiles.iter().all(|tile| tile.kind == "button"));
    }

    #[test]
    fn empty_device_config_gives_an_empty_profile() {
        let profile = parse_profile("{}").unwrap();

        assert!(profile.tiles.is_empty());
        assert_eq!(profile.format, FORMAT);
    }

    #[test]
    fn arduino_pins_skip_empty_macros_and_unusable_pins() {
        let json = r#"{"profile_name":"FlexPad-DCA1469FC11","pins":{"1":"","15":"MEDIA:mute","35":"P:f12,R:f12","36":"","4":"P:win,P:e,R:e,R:win"}}"#;

        let profile = parse_profile(json).unwrap();

        assert_eq!(profile.profile_name, "FlexPad-DCA1469FC11");
        assert_eq!(profile.tiles.len(), 3);
        let unusable = profile
            .tiles
            .iter()
            .find(|tile| tile.macro_for("press") == legacy("P:F12,R:F12"))
            .unwrap();
        assert_eq!(unusable.pin("switch"), None);
        assert!(
            profile
                .tiles
                .iter()
                .any(|tile| tile.pin("switch") == Some(4))
        );
    }

    #[test]
    fn migrates_tiles_with_a_pins_map() {
        let json = r#"{"profile_name":"Test","tiles":[
            {"id":"t1","display_name":"Media Mute","tile_type":"Button","pins":{"15":"MEDIA:mute"},"x":90.0,"y":120.0},
            {"id":"t2","display_name":"F12","tile_type":"Button","pins":{"42":"P:f12,R:f12"},"x":450.0,"y":180.0}
        ]}"#;

        let profile = parse_profile(json).unwrap();

        assert_eq!(profile.tiles[0].pin("switch"), Some(15));
        assert_eq!(profile.tiles[0].macro_for("press"), legacy("MEDIA:mute"));
        assert_eq!(profile.tiles[1].pin("switch"), Some(42));
        assert_eq!(profile.tiles[1].macro_for("press"), legacy("P:F12,R:F12"));
    }

    #[test]
    fn format_2_profiles_get_position_names_and_keep_their_layout() {
        let json = r#"{"format":2,"profile_name":"P","keyboard_layout":"de","tiles":[{"id":"t","display_name":"T","component":"button","kind":"button","pins":{"switch":4},"functions":{"press":"P:lctrl,P:z,R:z,R:lctrl"},"x":0.0,"y":0.0}]}"#;

        let profile = parse_profile(json).unwrap();

        assert_eq!(profile.keyboard_layout, KeyboardLayout::De);
        assert_eq!(
            profile.tiles[0].macro_for("press"),
            legacy("P:ControlLeft,P:KeyZ,R:KeyZ,R:ControlLeft")
        );
        assert!(
            profile
                .to_json()
                .unwrap()
                .contains(r#""keyboard_layout":"de""#)
        );
    }

    #[test]
    fn profiles_without_layout_use_us() {
        let profile = parse_profile(r#"{"format":2,"profile_name":"P","tiles":[]}"#).unwrap();

        assert_eq!(profile.keyboard_layout, KeyboardLayout::Us);
    }

    #[test]
    fn rejects_unknown_legacy_tile_types() {
        let json = r#"{"tiles":[{"id":"t","display_name":"T","tile_type":"Joystick","functions":{},"x":0,"y":0}]}"#;

        assert!(parse_profile(json).is_err());
    }
}

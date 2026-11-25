use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::config::app_dirs;
use crate::constants::MEDIA_ACTIONS;
use crate::i18n;
use crate::keys;
use crate::macros::{self, Macro};

mod bundled {
    include!(concat!(env!("OUT_DIR"), "/components.rs"));
}

// Constants

pub const DEFAULT_COMPONENT: &str = "button";
const FOLDER: &str = "components";
const EXTENSION: &str = "json";
const MAX_FILE_SIZE: u64 = 64 * 1024;
const MAX_ID_LENGTH: usize = 32;

// Types

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinRole {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DefaultMacro {
    Chord(Vec<String>),
    Media(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Function {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub default_macro: Option<DefaultMacro>,
}

impl Function {
    pub fn default_macro(&self) -> Macro {
        match &self.default_macro {
            Some(DefaultMacro::Chord(chord)) => macros::chord(
                &chord
                    .iter()
                    .map(|key| keys::code_for(key).unwrap_or(key))
                    .collect::<Vec<_>>(),
            ),
            Some(DefaultMacro::Media(name)) => macros::media(name),
            None => Macro::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParameterKind {
    Number { min: u32, max: u32, default: u32 },
    Choice { options: Vec<u32>, default: u32 },
    Flag { default: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Parameter {
    pub id: String,
    pub label: String,
    #[serde(flatten)]
    pub kind: ParameterKind,
}

impl Parameter {
    pub fn default_value(&self) -> serde_json::Value {
        match self.kind {
            ParameterKind::Number { default, .. } | ParameterKind::Choice { default, .. } => {
                default.into()
            }
            ParameterKind::Flag { default } => default.into(),
        }
    }

    pub fn accepts(&self, value: &serde_json::Value) -> bool {
        match &self.kind {
            ParameterKind::Number { min, max, .. } => value
                .as_u64()
                .is_some_and(|number| (u64::from(*min)..=u64::from(*max)).contains(&number)),
            ParameterKind::Choice { options, .. } => value
                .as_u64()
                .is_some_and(|number| options.iter().any(|&option| u64::from(option) == number)),
            ParameterKind::Flag { .. } => value.is_boolean(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub pins: Vec<PinRole>,
    pub functions: Vec<Function>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

impl Component {
    pub fn parameter(&self, id: &str) -> Option<&Parameter> {
        self.parameters.iter().find(|parameter| parameter.id == id)
    }
}

impl std::fmt::Display for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&i18n::text(&self.name))
    }
}

// Kinds

struct Kind {
    id: &'static str,
    pins: &'static [&'static str],
    functions: &'static [KindFunction],
    parameters: &'static [(&'static str, Limit)],
}

struct KindFunction {
    id: &'static str,
    pins: &'static [&'static str],
    required: bool,
}

enum Limit {
    Number { max: u32 },
    Choice(&'static [u32]),
    Flag,
}

const MAX_DEBOUNCE_MS: u32 = 200;
const STEPS_PER_DETENT: &[u32] = &[1, 2, 4];

const KINDS: &[Kind] = &[
    Kind {
        id: "button",
        pins: &["switch"],
        functions: &[KindFunction {
            id: "press",
            pins: &["switch"],
            required: true,
        }],
        parameters: &[(
            "debounce_ms",
            Limit::Number {
                max: MAX_DEBOUNCE_MS,
            },
        )],
    },
    Kind {
        id: "encoder",
        pins: &["a", "b", "switch"],
        functions: &[
            KindFunction {
                id: "turn_right",
                pins: &["a", "b"],
                required: true,
            },
            KindFunction {
                id: "turn_left",
                pins: &["a", "b"],
                required: true,
            },
            KindFunction {
                id: "press",
                pins: &["switch"],
                required: false,
            },
        ],
        parameters: &[
            ("steps_per_detent", Limit::Choice(STEPS_PER_DETENT)),
            ("reverse", Limit::Flag),
            (
                "debounce_ms",
                Limit::Number {
                    max: MAX_DEBOUNCE_MS,
                },
            ),
        ],
    },
];

// Library

pub fn all() -> &'static [Component] {
    static LIBRARY: OnceLock<Vec<Component>> = OnceLock::new();
    LIBRARY.get_or_init(|| load(Some(&user_directory())))
}

pub fn find(id: &str) -> Option<&'static Component> {
    all().iter().find(|component| component.id == id)
}

pub fn default_component() -> &'static Component {
    find(DEFAULT_COMPONENT)
        .or_else(|| all().first())
        .expect("the built-in component definitions are valid")
}

fn user_directory() -> PathBuf {
    app_dirs().config.join(FOLDER)
}

fn load(user_directory: Option<&Path>) -> Vec<Component> {
    let mut components: Vec<Component> = bundled::BUNDLED
        .iter()
        .filter_map(|&(name, content)| parse(name, content).ok())
        .collect();
    for (name, content) in user_directory.map(read_directory).unwrap_or_default() {
        if let Ok(component) = parse(&name, &content)
            && !components.iter().any(|known| known.id == component.id)
        {
            components.push(component);
        }
    }
    components
}

fn read_directory(directory: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut files: Vec<(String, String)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(EXTENSION))
        })
        .filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_owned();
            let metadata = fs::metadata(&path).ok()?;
            if !metadata.is_file() || metadata.len() > MAX_FILE_SIZE {
                return None;
            }
            Some((name, fs::read_to_string(&path).ok()?))
        })
        .collect();
    files.sort();
    files
}

// Validation

pub fn parse(file_name: &str, content: &str) -> Result<Component, String> {
    let component: Component = serde_json::from_str(content).map_err(|error| error.to_string())?;
    validate(&component)?;
    if component.id != file_name {
        return Err(format!(
            "the file name must be the id: {}.{EXTENSION}",
            component.id
        ));
    }
    Ok(component)
}

fn validate(component: &Component) -> Result<(), String> {
    if !is_valid_id(&component.id) {
        return Err(format!("invalid id '{}'", component.id));
    }
    if component.name.trim().is_empty() {
        return Err("the name is empty".to_string());
    }
    let kind = KINDS
        .iter()
        .find(|kind| kind.id == component.kind)
        .ok_or_else(|| format!("unknown kind '{}'", component.kind))?;
    validate_pins(component, kind)?;
    validate_functions(component, kind)?;
    validate_parameters(component, kind)
}

fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LENGTH
        && id.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '-'
                || character == '_'
        })
}

fn validate_pins(component: &Component, kind: &Kind) -> Result<(), String> {
    for (index, role) in component.pins.iter().enumerate() {
        if !kind.pins.contains(&role.id.as_str()) {
            return Err(format!("unknown pin '{}' for kind {}", role.id, kind.id));
        }
        if component.pins[..index]
            .iter()
            .any(|other| other.id == role.id)
        {
            return Err(format!("pin '{}' is listed twice", role.id));
        }
        if role.label.trim().is_empty() {
            return Err(format!("pin '{}' has no label", role.id));
        }
    }
    Ok(())
}

fn validate_functions(component: &Component, kind: &Kind) -> Result<(), String> {
    for (index, function) in component.functions.iter().enumerate() {
        let known = kind
            .functions
            .iter()
            .find(|known| known.id == function.id)
            .ok_or_else(|| format!("unknown function '{}' for kind {}", function.id, kind.id))?;
        if component.functions[..index]
            .iter()
            .any(|other| other.id == function.id)
        {
            return Err(format!("function '{}' is listed twice", function.id));
        }
        if function.label.trim().is_empty() {
            return Err(format!("function '{}' has no label", function.id));
        }
        if let Some(pin) = known
            .pins
            .iter()
            .find(|pin| !component.pins.iter().any(|role| role.id == **pin))
        {
            return Err(format!("function '{}' needs pin '{pin}'", function.id));
        }
        validate_default_macro(function)?;
    }
    if let Some(missing) = kind.functions.iter().find(|known| {
        known.required
            && !component
                .functions
                .iter()
                .any(|function| function.id == known.id)
    }) {
        return Err(format!("function '{}' is missing", missing.id));
    }
    Ok(())
}

fn validate_default_macro(function: &Function) -> Result<(), String> {
    match &function.default_macro {
        Some(DefaultMacro::Chord(chord)) => {
            if chord.is_empty() {
                return Err(format!("the chord of '{}' is empty", function.id));
            }
            if let Some(key) = chord.iter().find(|key| keys::code_for(key).is_none()) {
                return Err(format!("unknown key '{key}' in '{}'", function.id));
            }
            Ok(())
        }
        Some(DefaultMacro::Media(name)) => {
            if MEDIA_ACTIONS.iter().any(|(_, known)| known == name) {
                Ok(())
            } else {
                Err(format!("unknown media key '{name}' in '{}'", function.id))
            }
        }
        None => Ok(()),
    }
}

fn validate_parameters(component: &Component, kind: &Kind) -> Result<(), String> {
    for (index, parameter) in component.parameters.iter().enumerate() {
        let (_, limit) = kind
            .parameters
            .iter()
            .find(|(id, _)| *id == parameter.id)
            .ok_or_else(|| format!("unknown parameter '{}' for kind {}", parameter.id, kind.id))?;
        if component.parameters[..index]
            .iter()
            .any(|other| other.id == parameter.id)
        {
            return Err(format!("parameter '{}' is listed twice", parameter.id));
        }
        if parameter.label.trim().is_empty() {
            return Err(format!("parameter '{}' has no label", parameter.id));
        }
        let fits = match (&parameter.kind, limit) {
            (ParameterKind::Number { min, max, default }, Limit::Number { max: limit }) => {
                min <= default && default <= max && max <= limit
            }
            (ParameterKind::Choice { options, default }, Limit::Choice(allowed)) => {
                !options.is_empty()
                    && options.iter().all(|option| allowed.contains(option))
                    && options.contains(default)
            }
            (ParameterKind::Flag { .. }, Limit::Flag) => true,
            _ => false,
        };
        if !fits {
            return Err(format!(
                "parameter '{}' has the wrong type or range",
                parameter.id
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRMWARE_CONFIG: &str = include_str!("../../flexpad-firmware/src/config.rs");
    const FIRMWARE_BUTTONS: &str = include_str!("../../flexpad-firmware/src/buttons.rs");
    const ENGLISH: &str = include_str!("../lang/en.lang");
    const DOCUMENT: &str = include_str!("../../docs/components.md");

    fn component(id: &str) -> &'static Component {
        find(id).unwrap()
    }

    fn temp_directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("flexpad-components-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn definition(id: &str, kind: &str, extra: &str) -> String {
        let (pins, functions) = match kind {
            "encoder" => (
                r#"[{"id":"a","label":"A"},{"id":"b","label":"B"}]"#,
                r#"[{"id":"turn_right","label":"Right"},{"id":"turn_left","label":"Left"}]"#,
            ),
            _ => (
                r#"[{"id":"switch","label":"Pin"}]"#,
                r#"[{"id":"press","label":"Press"}]"#,
            ),
        };
        format!(
            r#"{{"id":"{id}","name":"Test","kind":"{kind}","pins":{pins},"functions":{functions}{extra}}}"#
        )
    }

    // Built-in files

    #[test]
    fn built_in_files_are_valid() {
        assert!(!bundled::BUNDLED.is_empty());
        for (name, content) in bundled::BUNDLED {
            parse(name, content).unwrap_or_else(|error| panic!("{name}.json: {error}"));
        }
        assert_eq!(all().len(), bundled::BUNDLED.len());
        assert_eq!(default_component().id, DEFAULT_COMPONENT);
    }

    #[test]
    fn built_in_labels_are_translated() {
        let keys: Vec<&str> = ENGLISH
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(key, _)| key.trim())
            .collect();
        for component in load(None) {
            let labels = [component.name.as_str()]
                .into_iter()
                .chain(component.pins.iter().map(|role| role.label.as_str()))
                .chain(
                    component
                        .functions
                        .iter()
                        .map(|function| function.label.as_str()),
                )
                .chain(
                    component
                        .parameters
                        .iter()
                        .map(|parameter| parameter.label.as_str()),
                );
            for label in labels {
                assert!(keys.contains(&label), "{}: {label}", component.id);
            }
        }
    }

    #[test]
    fn built_in_components_keep_their_defaults() {
        let button = component("button");
        let encoder = component("ec11");

        assert_eq!(button.kind, "button");
        assert_eq!(
            button.functions[0].default_macro(),
            macros::chord(&["ControlLeft", "KeyC"])
        );
        assert_eq!(encoder.kind, "encoder");
        assert_eq!(
            encoder
                .pins
                .iter()
                .map(|role| role.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b", "switch"]
        );
        assert_eq!(
            encoder
                .functions
                .iter()
                .map(|function| function.id.as_str())
                .collect::<Vec<_>>(),
            ["turn_right", "turn_left", "press"]
        );
        assert_eq!(encoder.functions[2].default_macro(), macros::media("mute"));
    }

    #[test]
    fn defaults_are_accepted() {
        for component in all() {
            for parameter in &component.parameters {
                assert!(
                    parameter.accepts(&parameter.default_value()),
                    "{}",
                    parameter.id
                );
            }
        }
    }

    // Firmware

    #[test]
    fn kinds_match_the_firmware() {
        for kind in KINDS {
            let quoted = |names: &mut dyn Iterator<Item = &str>| {
                names
                    .map(|name| format!("{name:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let pins = quoted(&mut kind.pins.iter().copied());
            let functions = quoted(&mut kind.functions.iter().map(|function| function.id));
            let call = format!("self.check_names(&[{pins}], &[{functions}])");
            assert!(FIRMWARE_CONFIG.contains(&call), "{call}");
            assert!(FIRMWARE_CONFIG.contains(&format!("kind.eq_str({:?})", kind.id)));
            for (parameter, _) in kind.parameters {
                assert!(FIRMWARE_CONFIG.contains(&format!("name.eq_str({parameter:?})")));
            }
        }
        assert!(FIRMWARE_BUTTONS.contains(&format!(
            "pub const MAX_DEBOUNCE_MS: u8 = {MAX_DEBOUNCE_MS};"
        )));
        let steps = STEPS_PER_DETENT
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(FIRMWARE_CONFIG.contains(&format!("value @ ({steps})")));
    }

    // Parameters

    #[test]
    fn parameters_check_their_range() {
        let debounce = component("button").parameter("debounce_ms").unwrap();
        let steps = component("ec11").parameter("steps_per_detent").unwrap();
        let reverse = component("ec11").parameter("reverse").unwrap();

        assert!(debounce.accepts(&serde_json::json!(200)));
        assert!(!debounce.accepts(&serde_json::json!(201)));
        assert!(!debounce.accepts(&serde_json::json!(-1)));
        assert!(steps.accepts(&serde_json::json!(2)));
        assert!(!steps.accepts(&serde_json::json!(3)));
        assert!(reverse.accepts(&serde_json::json!(true)));
        assert!(!reverse.accepts(&serde_json::json!(1)));
    }

    #[test]
    fn finds_components_by_id() {
        assert_eq!(
            find("ec11").map(|found| found.kind.as_str()),
            Some("encoder")
        );
        assert_eq!(find("joystick"), None);
    }

    // Validation

    #[test]
    fn accepts_minimal_definitions() {
        let encoder = parse("ky-040", &definition("ky-040", "encoder", "")).unwrap();
        assert!(encoder.parameters.is_empty());
        assert!(encoder.functions[0].default_macro().actions.is_empty());

        let extra = r#","parameters":[{"id":"steps_per_detent","label":"Steps","type":"choice","options":[2,4],"default":2}]"#;
        let encoder = parse("ec12", &definition("ec12", "encoder", extra)).unwrap();
        assert_eq!(
            encoder
                .parameter("steps_per_detent")
                .unwrap()
                .default_value(),
            2
        );
    }

    #[test]
    fn rejects_invalid_definitions() {
        let rejected = [
            ("other", definition("button2", "button", "")),
            ("Big", definition("Big", "button", "")),
            ("toggle", definition("toggle", "switch", "")),
            ("pins", r#"{"id":"pins","name":"T","kind":"button","pins":[{"id":"x","label":"X"}],"functions":[{"id":"press","label":"P"}]}"#.to_string()),
            ("missing", r#"{"id":"missing","name":"T","kind":"encoder","pins":[{"id":"a","label":"A"},{"id":"b","label":"B"}],"functions":[{"id":"turn_right","label":"R"}]}"#.to_string()),
            ("press", r#"{"id":"press","name":"T","kind":"encoder","pins":[{"id":"a","label":"A"},{"id":"b","label":"B"}],"functions":[{"id":"turn_right","label":"R"},{"id":"turn_left","label":"L"},{"id":"press","label":"P"}]}"#.to_string()),
            ("twice", r#"{"id":"twice","name":"T","kind":"button","pins":[{"id":"switch","label":"P"},{"id":"switch","label":"P"}],"functions":[{"id":"press","label":"P"}]}"#.to_string()),
            ("range", definition("range", "button", r#","parameters":[{"id":"debounce_ms","label":"D","type":"number","min":0,"max":255,"default":10}]"#)),
            ("steps", definition("steps", "encoder", r#","parameters":[{"id":"steps_per_detent","label":"S","type":"choice","options":[3],"default":3}]"#)),
            ("type", definition("type", "encoder", r#","parameters":[{"id":"reverse","label":"R","type":"number","min":0,"max":1,"default":0}]"#)),
            ("key", r#"{"id":"key","name":"T","kind":"button","pins":[{"id":"switch","label":"P"}],"functions":[{"id":"press","label":"P","default_macro":{"chord":["NoSuchKey"]}}]}"#.to_string()),
            ("media", r#"{"id":"media","name":"T","kind":"button","pins":[{"id":"switch","label":"P"}],"functions":[{"id":"press","label":"P","default_macro":{"media":"louder"}}]}"#.to_string()),
            ("field", definition("field", "button", r#","colour":"red""#)),
            ("json", "{".to_string()),
        ];
        for (name, content) in rejected {
            assert!(parse(name, &content).is_err(), "{name}");
        }
    }

    #[test]
    fn documented_example_is_valid() {
        let example = DOCUMENT
            .split("```json\n")
            .nth(1)
            .and_then(|rest| rest.split("```").next())
            .unwrap();
        let component = parse("panel-encoder", example).unwrap();
        assert!(component.pins.iter().all(|role| role.id != "switch"));
    }

    // User files

    #[test]
    fn user_files_add_components_but_keep_built_in_ones() {
        let directory = temp_directory("user");
        fs::write(
            directory.join("ky-040.json"),
            definition("ky-040", "encoder", ""),
        )
        .unwrap();
        fs::write(
            directory.join("button.json"),
            definition("button", "encoder", ""),
        )
        .unwrap();
        fs::write(directory.join("broken.json"), "{").unwrap();
        fs::write(directory.join("notes.txt"), "text").unwrap();
        let components = load(Some(&directory));
        fs::remove_dir_all(&directory).unwrap();

        let ids: Vec<&str> = components.iter().map(|found| found.id.as_str()).collect();
        assert_eq!(ids, ["button", "ec11", "ky-040"]);
        assert_eq!(components[0].kind, "button");
        assert_eq!(load(Some(&directory)).len(), bundled::BUNDLED.len());
    }
}

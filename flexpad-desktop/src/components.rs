// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinRole {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Function {
    pub id: &'static str,
    pub label: &'static str,
    pub default_macro: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterKind {
    Number {
        min: u32,
        max: u32,
        default: u32,
    },
    Choice {
        options: &'static [u32],
        default: u32,
    },
    Flag {
        default: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameter {
    pub id: &'static str,
    pub label: &'static str,
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
        match self.kind {
            ParameterKind::Number { min, max, .. } => value
                .as_u64()
                .is_some_and(|number| (u64::from(min)..=u64::from(max)).contains(&number)),
            ParameterKind::Choice { options, .. } => value
                .as_u64()
                .is_some_and(|number| options.iter().any(|&option| u64::from(option) == number)),
            ParameterKind::Flag { .. } => value.is_boolean(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: &'static str,
    pub pins: &'static [PinRole],
    pub functions: &'static [Function],
    pub parameters: &'static [Parameter],
}

impl Component {
    pub fn parameter(&self, id: &str) -> Option<&'static Parameter> {
        self.parameters.iter().find(|parameter| parameter.id == id)
    }
}

impl std::fmt::Display for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

// Components

const DEBOUNCE: Parameter = Parameter {
    id: "debounce_ms",
    label: "Debounce (ms)",
    kind: ParameterKind::Number {
        min: 0,
        max: 200,
        default: 10,
    },
};

pub const BUTTON: Component = Component {
    id: "button",
    name: "Button",
    kind: "button",
    pins: &[PinRole {
        id: "switch",
        label: "Pin",
    }],
    functions: &[Function {
        id: "press",
        label: "Press",
        default_macro: "P:lctrl,P:c,D:50,R:c,R:lctrl",
    }],
    parameters: &[DEBOUNCE],
};

pub const EC11: Component = Component {
    id: "ec11",
    name: "EC11 Rotary Encoder",
    kind: "encoder",
    pins: &[
        PinRole {
            id: "a",
            label: "Pin A",
        },
        PinRole {
            id: "b",
            label: "Pin B",
        },
        PinRole {
            id: "switch",
            label: "Push button pin",
        },
    ],
    functions: &[
        Function {
            id: "turn_right",
            label: "Turn right",
            default_macro: "MEDIA:vol_up",
        },
        Function {
            id: "turn_left",
            label: "Turn left",
            default_macro: "MEDIA:vol_down",
        },
        Function {
            id: "press",
            label: "Press",
            default_macro: "MEDIA:mute",
        },
    ],
    parameters: &[
        Parameter {
            id: "steps_per_detent",
            label: "Steps per detent",
            kind: ParameterKind::Choice {
                options: &[1, 2, 4],
                default: 4,
            },
        },
        Parameter {
            id: "reverse",
            label: "Reverse direction",
            kind: ParameterKind::Flag { default: false },
        },
        DEBOUNCE,
    ],
};

pub const ALL: &[Component] = &[BUTTON, EC11];

pub fn find(id: &str) -> Option<&'static Component> {
    ALL.iter().find(|component| component.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Table

    #[test]
    fn ids_are_unique() {
        for (index, component) in ALL.iter().enumerate() {
            assert!(
                ALL[index + 1..]
                    .iter()
                    .all(|other| other.id != component.id)
            );
            for (role_index, role) in component.pins.iter().enumerate() {
                assert!(
                    component.pins[role_index + 1..]
                        .iter()
                        .all(|other| other.id != role.id)
                );
            }
            for (function_index, function) in component.functions.iter().enumerate() {
                assert!(
                    component.functions[function_index + 1..]
                        .iter()
                        .all(|other| other.id != function.id)
                );
            }
        }
    }

    #[test]
    fn kinds_match_the_firmware() {
        assert_eq!(BUTTON.kind, "button");
        assert_eq!(EC11.kind, "encoder");
        assert_eq!(
            EC11.pins.iter().map(|role| role.id).collect::<Vec<_>>(),
            ["a", "b", "switch"]
        );
        assert_eq!(
            EC11.functions
                .iter()
                .map(|function| function.id)
                .collect::<Vec<_>>(),
            ["turn_right", "turn_left", "press"]
        );
    }

    #[test]
    fn defaults_are_accepted() {
        for component in ALL {
            for parameter in component.parameters {
                assert!(
                    parameter.accepts(&parameter.default_value()),
                    "{}",
                    parameter.id
                );
            }
        }
    }

    // Parameters

    #[test]
    fn parameters_check_their_range() {
        let debounce = BUTTON.parameter("debounce_ms").unwrap();
        let steps = EC11.parameter("steps_per_detent").unwrap();
        let reverse = EC11.parameter("reverse").unwrap();

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
        assert_eq!(find("ec11"), Some(&EC11));
        assert_eq!(find("joystick"), None);
    }
}

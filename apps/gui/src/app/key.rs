use crate::app::Scope;
use crate::app::UserCommand;
use egui::{Key, Modifiers};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(PartialEq, Eq, Hash, Debug, Clone, derive_more::From)]
pub struct KeyShortcut(pub egui::KeyboardShortcut);

impl std::ops::Deref for KeyShortcut {
    type Target = egui::KeyboardShortcut;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for KeyShortcut {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<KeyShortcut>().map_err(de::Error::custom)
    }
}

impl Serialize for KeyShortcut {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.format_string())
    }
}

impl FromStr for KeyShortcut {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // NOTE, `C--` is not allowed here, instead, one should write `C-Minus`
        let parts: std::vec::Vec<&str> = s.split("-").collect();
        if parts.is_empty() {
            return Err("Empty key string".to_string());
        }

        let mut modifiers = Modifiers::NONE;
        let mut key: Option<Key> = None;

        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;

            macro_rules! set_mod {
                ($field:ident) => {{
                    if modifiers.$field {
                        return Err(format!("Duplicate modifier: {}", part));
                    }
                    modifiers.$field = true;
                }};
            }

            if !is_last {
                match *part {
                    "C" | "Ctrl" | "Control" => set_mod!(ctrl),
                    "S" | "Shift" => set_mod!(shift),
                    "A" | "Alt" | "Opt" | "Option" => set_mod!(alt),
                    "M" | "Meta" | "Cmd" | "Command" | "Super" | "Win" => {
                        set_mod!(command)
                    }
                    _ => return Err(format!("Unknown modifier: {}", part)),
                }
            } else {
                key = egui::Key::from_name(&part);
            }
        }

        match key {
            Some(k) => Ok(Self(egui::KeyboardShortcut::new(modifiers, k))),
            None => Err("No key specified".to_string()),
        }
    }
}

impl KeyShortcut {
    pub fn format_string(&self) -> String {
        let mut s = String::new();
        if self.0.modifiers.ctrl {
            s.push_str("Ctrl-");
        }
        if self.0.modifiers.alt {
            s.push_str("Alt-");
        }
        if self.0.modifiers.shift {
            s.push_str("Shift-");
        }
        if self.0.modifiers.command {
            s.push_str("Cmd-");
        }
        s.push_str(self.0.logical_key.name());
        s
    }
}

pub type KeyConfig = HashMap<Scope, HashMap<KeyShortcut, UserCommand>>;

pub fn merge_key_config(base: &mut KeyConfig, delta: KeyConfig) {
    for (scope, shortcuts) in delta {
        let base_shortcuts = base.entry(scope).or_default();
        base_shortcuts.extend(shortcuts);
    }
}

#[macro_export]
macro_rules! key_config {
    // Pattern: Scope => { "Key" => Command, ... }, ...
    (
        $(
            $scope:expr => {
                $( $key_str:literal => $cmd:expr ),* $(,)?
            }
        ),* $(,)?
    ) => {{
        let mut config = KeyConfig::new();

        $(
            let mut scope_map = std::collections::HashMap::new();
            $(
                let key: KeyShortcut = $key_str
                    .parse()
                    .expect(concat!("Invalid key binding string: ", $key_str));

                // Check for duplicates within the same scope (optional safety)
                if scope_map.insert(key, $cmd).is_some() {
                    panic!("Duplicate key binding defined for scope: {:?}", $scope);
                }
            )*
            config.insert($scope, scope_map);
        )*

        config
    }};
}

pub fn default_key_config() -> KeyConfig {
    key_config! {
        Scope::Global => {
            "Ctrl-Q" => UserCommand::QuitApplication,
            "F11" => UserCommand::ToggleFullScreen,
            "Tab" => UserCommand::ToggleSearchMode,
        },
        Scope::Main => {
            "Down" => UserCommand::NextItem,
            "Up" => UserCommand::PrevItem,
            "Ctrl-N" => UserCommand::NextItem,
            "Ctrl-P" => UserCommand::PrevItem,
            "Ctrl-J" => UserCommand::NextItem,
            "Ctrl-K" => UserCommand::PrevItem,
        },
        Scope::SearchBarCompletion => {
            "Down" => UserCommand::NextItem,
            "Up" => UserCommand::PrevItem,
            "Ctrl-N" => UserCommand::NextItem,
            "Ctrl-P" => UserCommand::PrevItem,
            "Ctrl-J" => UserCommand::NextItem,
            "Ctrl-K" => UserCommand::PrevItem,
            "Enter" => UserCommand::ApplyCompletion,
            "Esc" => UserCommand::CancelCompletion,
        },
        Scope::SearchBar => {
            "Enter" => UserCommand::StartSearch
        }
    }
}

pub struct KeyHandler(KeyConfig);

impl KeyHandler {
    pub fn new(key_config: KeyConfig) -> Self {
        Self(key_config)
    }
    
    pub fn handle(
        &self,
        ctx: &egui::Context,
        current_scope: &Scope,
    ) -> Vec<(Scope, UserCommand)> {
        let mut matched = Vec::new();

        for scope in current_scope.hierarchy() {
            if let Some(bindings) = self.0.get(&scope) {
                for (key_shortcut, user_command) in bindings {
                    if ctx.input_mut(|i| i.consume_shortcut(&key_shortcut.0)) {
                        matched.push((scope.clone(), user_command.clone()));
                    }
                }
            }
        }

        matched
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        "Down",
        KeyShortcut(egui::KeyboardShortcut::new(Modifiers::NONE, Key::ArrowDown))
    )]
    #[case(
        "Alt-Down",
        KeyShortcut(egui::KeyboardShortcut::new(Modifiers::ALT, Key::ArrowDown))
    )]
    #[case("Ctrl-Alt-A", KeyShortcut(egui::KeyboardShortcut::new(Modifiers::ALT | Modifiers::CTRL, Key::A)))]
    fn test_from_str(#[case] string: &str, #[case] expected_key_shortcut: KeyShortcut) {
        let key_shortcut = string.parse::<KeyShortcut>();
        assert!(key_shortcut.is_ok());
        let key_shortcut_unwrapped = key_shortcut.unwrap();
        assert_eq!(expected_key_shortcut, key_shortcut_unwrapped);
    }

    #[rstest]
    #[case("C--", "Unknown modifier: ")]
    #[case("C-*", "No key specified")]
    #[case("C-*-A", "Unknown modifier: *")]
    fn test_from_str_error_cases(#[case] string: &str, #[case] expected_error_str: &str) {
        let key_shortcut = string.parse::<KeyShortcut>();
        assert!(key_shortcut.is_err());
        let error_str = key_shortcut.unwrap_err();
        assert_eq!(expected_error_str, error_str);
    }

    #[rstest]
    #[case(
        "Down",
        KeyShortcut(egui::KeyboardShortcut::new(Modifiers::NONE, Key::ArrowDown))
    )]
    #[case(
        "Alt-Down",
        KeyShortcut(egui::KeyboardShortcut::new(Modifiers::ALT, Key::ArrowDown))
    )]
    #[case("Ctrl-Alt-A", KeyShortcut(egui::KeyboardShortcut::new(Modifiers::ALT | Modifiers::CTRL, Key::A)))]
    fn test_format_string(
        #[case] expected_format_string: &str,
        #[case] key_shortcut: KeyShortcut,
    ) {
        assert_eq!(expected_format_string, key_shortcut.format_string());
    }
}

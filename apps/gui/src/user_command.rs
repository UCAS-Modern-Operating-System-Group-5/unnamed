use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UserCommand {
    QuitApplication,
    NextItem,
    PrevItem
}


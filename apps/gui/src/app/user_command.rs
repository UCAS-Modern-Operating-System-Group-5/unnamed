use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UserCommand {
    QuitApplication,
    ToggleFullScreen,
    ToggleSearchMode,
    
    NextItem,
    PrevItem,
    
    CancelCompletion,
    ApplyCompletion,
    
    StartSearch
}

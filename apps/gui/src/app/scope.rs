use serde::Deserialize;

#[derive(Hash, Eq, Default, PartialEq, Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    #[default]
    Global,
    Main,
    SearchBar,
    SearchBarCompletion,
}

impl Scope {
    /// Returns scopes from most specific (self) to least specific (Global)
    /// This defines the "bubbling" order for key events
    pub fn hierarchy(&self) -> Vec<Scope> {
        match self {
            Scope::SearchBarCompletion => vec![
                Scope::SearchBarCompletion,
                Scope::SearchBar,
                Scope::Main,
                Scope::Global,
            ],
            Scope::SearchBar => vec![Scope::SearchBar, Scope::Main, Scope::Global],
            Scope::Main => vec![Scope::Main, Scope::Global],
            Scope::Global => vec![Scope::Global],
        }
    }
}

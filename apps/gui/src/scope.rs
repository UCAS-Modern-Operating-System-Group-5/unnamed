use serde::Deserialize;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    Global,
    Main,
    SearchbarMain // Main -> SearchBar
}

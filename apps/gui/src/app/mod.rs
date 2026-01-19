mod main;
mod config;
mod user_command;
mod scope;
mod key;
use serde::{Serialize, Deserialize};
use strum::{EnumIter, EnumCount};

pub use main::{App, Request, Response};
pub use config::AppConfig;
pub use user_command::UserCommand;
pub use scope::Scope;
pub use key::{KeyConfig, KeyShortcut, KeyHandler, merge_key_config, default_key_config};

#[derive(
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    strum::Display,
    EnumIter,
    EnumCount,
    Clone,
)]
pub enum SortMode {
    #[default]
    Alphabetical,
    ReverseAlphabetical,
    AccessedTime,
    CreatedTime,
    ModifiedTime,
    Extension,
    /// Sort by AI relevance score
    Relevance,
}

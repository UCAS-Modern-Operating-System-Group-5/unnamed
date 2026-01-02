pub type Result<T> = color_eyre::Result<T>;
#[allow(dead_code)]
pub type Error = color_eyre::eyre::Report;

pub use color_eyre::eyre::WrapErr;
pub use color_eyre::eyre::OptionExt;
pub use color_eyre::eyre::eyre as error;


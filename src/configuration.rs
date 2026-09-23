use serde::Deserialize;
use crate::backend::Backend;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub config: Config,

    #[serde(flatten)]
    pub astromart: Option<Backend>,

    #[serde(flatten)]
    pub cloudynights: Option<Backend>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub gotify_server: String,
    pub gotify_key: String,
}

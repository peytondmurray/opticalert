use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Site {
    pub pages: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub config: Config,
    pub astromart: Option<Site>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub gotify_server: String,
    pub gotify_key: String,
}

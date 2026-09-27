use config::Config;
use directories::ProjectDirs;
use fs::OpenOptions;
use std::io::Write;
use std::{error::Error, path::Path};
use std::{fs, path::PathBuf};
use tracing::info;

use crate::backend::Backend;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    pub config: OtherOptions,

    #[serde(flatten)]
    pub astromart: Option<Backend>,

    #[serde(flatten)]
    pub cloudynights: Option<Backend>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OtherOptions {
    pub gotify_server: String,
    pub gotify_key: String,
}

pub fn ensure_config_exists(path: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    if !path.exists() {
        let parent = path
            .parent()
            .ok_or(format!("Can't get parent of path: {path:?}"))?;
        if !parent.exists() {
            info!("Creating path: {:?}", parent);
            fs::create_dir_all(parent)
                .map_err(|err| format!("Can't create path: {parent:?}; reason: {err:?}"))?;
        }
        let cf = ConfigFile {
            config: OtherOptions {
                gotify_server: "<your server>".to_string(),
                gotify_key: "<your key>".to_string(),
            },
            astromart: {
                Some(Backend::Astromart {
                    pages: vec!["<your classifieds search>".to_string()],
                    headers: None,
                })
            },
            cloudynights: None,
        };

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|err| {
                format!("Unable to open config at {path:?} for writing.\nReason: {err}")
            })?;
        file.write_all(toml::to_string_pretty(&cf)?.to_string().as_bytes())
            .map_err(|err| format!("Unable to write config to {path:?}.\nReason: {err}"))?;
    }

    // Write a default config

    Ok(path)
}

pub fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(ProjectDirs::from("", "", "opticalert")
        .ok_or("Can't find project directory")?
        .config_dir()
        .to_path_buf()
        .join("config.toml"))
}

impl ConfigFile {
    pub fn from_path(path: &Path) -> Result<Self, Box<dyn Error>> {
        let config_path = path.join("config.toml");

        let settings = Config::builder()
            .add_source(config::File::with_name(&config_path.to_string_lossy()))
            .add_source(config::Environment::with_prefix("OPTICALERT"))
            .build()?;

        Ok(settings.try_deserialize::<Self>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::home_dir;

    #[test]
    fn test_from_path() {}

    #[test]
    fn test_get_config_dir() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            get_config_path()?,
            home_dir()
                .ok_or("Can't get home directory")?
                .join(".config")
                .join("opticalert")
                .join("config.toml"),
        );
        Ok(())
    }
}

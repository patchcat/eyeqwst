use std::{collections::HashMap, fs, path::PathBuf};

use directories::BaseDirs;
use quaddlecl::model::channel::ChannelId;
use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = "eyeqwst/config.toml";

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub accounts: HashMap<String, HashMap<String, Account>>,
}

impl Config {
    pub fn load() -> Config {
        let Some(dirs) = BaseDirs::new() else {
            return Default::default()
        };
        let path = dirs.config_dir().join(CONFIG_PATH);
        let Ok(contents) = fs::read_to_string(&path) else {
            return Default::default()
        };

        let Ok(config) = toml::from_str(&contents) else {
            return Default::default();
        };

        config
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        let Some(dirs) = BaseDirs::new() else {
            log::warn!("could not find basedirs");
            return
        };

        let path = dirs.config_dir().join(CONFIG_PATH);

        let toml_str = match toml::to_string_pretty(&self) {
            Ok(x) => x,
            Err(e) => {
                log::warn!("could not serialize config: {e}");
                return
            }
        };


        if let Some(ancestor) = path.parent() {
            if let Err(e) = fs::create_dir_all(ancestor) {
                log::warn!("could not create {path}: {e}",
                           path = path.as_os_str().to_string_lossy());
                return
            }
        }

        if let Err(e) = fs::write(path, toml_str) {
            log::warn!("could not write config file: {e}");
            return
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Account {
    pub channels: Vec<Channel>
}

#[derive(Serialize, Deserialize)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
}



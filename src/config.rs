use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use directories::BaseDirs;
use quaddlecl::model::{channel::ChannelId, user::UserId};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use url::Url;

#[cfg(not(target_arch = "wasm32"))]
const CONFIG_PATH: &str = "eyeqwst/config.json";

#[serde_as]
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    #[serde_as(as = "HashMap<_, HashMap<DisplayFromStr, _>>")]
    pub accounts: HashMap<Url, HashMap<UserId, Account>>,
}

impl Config {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() -> Config {
        let Some(dirs) = BaseDirs::new() else {
            log::warn!("could not get basedirs");
            return Default::default();
        };
        let path = dirs.config_dir().join(CONFIG_PATH);
        let Ok(contents) = fs::read_to_string(path) else {
            log::warn!("could not read file");
            return Default::default();
        };

        let config = match serde_json::from_str(&contents) {
            Ok(x) => x,
            Err(e) => {
                log::warn!("error deserializing config: {e}");
                return Default::default();
            }
        };

        log::debug!("config: {config:?}");

        config
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Config {
        web_sys::window()
            .unwrap()
            .local_storage()
            .unwrap()
            .unwrap()
            .get_item("config")
            .unwrap()
            .and_then(|json| {
                serde_json::from_str(&json)
                    .inspect_err(|err| log::error!("deserialization error: {err}"))
                    .ok()
            })
            .unwrap_or_default()
    }

    pub fn get_account_config(&self, quaddle_url: &Url, user: UserId) -> Option<&Account> {
        self.accounts.get(quaddle_url)?.get(&user)
    }

    pub fn get_account_config_mut(&mut self, quaddle_url: &Url, user: UserId) -> &mut Account {
        self.accounts
            .entry(quaddle_url.clone())
            .or_default()
            .entry(user)
            .or_default()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&mut self) {
        let Some(dirs) = BaseDirs::new() else {
            log::warn!("could not find basedirs");
            return;
        };

        let path = dirs.config_dir().join(CONFIG_PATH);

        let toml_str = match serde_json::to_string_pretty(&self) {
            Ok(x) => x,
            Err(e) => {
                log::warn!("could not serialize config: {e}");
                return;
            }
        };

        if let Some(ancestor) = path.parent() {
            if let Err(e) = fs::create_dir_all(ancestor) {
                log::warn!(
                    "could not create {path}: {e}",
                    path = path.as_os_str().to_string_lossy()
                );
                return;
            }
        }

        if let Err(e) = fs::write(path, toml_str) {
            log::warn!("could not write config file: {e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save(&mut self) {
        let json_str = match serde_json::to_string_pretty(&self) {
            Ok(x) => x,
            Err(e) => {
                log::warn!("could not serialize config: {e}");
                return;
            }
        };
        web_sys::window()
            .unwrap()
            .local_storage()
            .unwrap()
            .unwrap()
            .set_item("config", &json_str)
            .unwrap()
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        self.save()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Account {
    pub channels: Vec<Channel>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
}

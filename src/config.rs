use std::{collections::HashMap, fs, path::PathBuf};

use directories::BaseDirs;
use quaddlecl::model::{channel::ChannelId, user::UserId};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use url::Url;

use crate::GatewayState;

const CONFIG_PATH: &str = "eyeqwst/config.toml";

#[serde_as]
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    #[serde_as(as = "HashMap<_, HashMap<DisplayFromStr, _>>")]
    pub accounts: HashMap<Url, HashMap<UserId, Account>>,
}

impl Config {
    pub fn load() -> Config {
        let Some(dirs) = BaseDirs::new() else {
            log::warn!("could not get basedirs");
            return Default::default();
        };
        let path = dirs.config_dir().join(CONFIG_PATH);
        let Ok(contents) = fs::read_to_string(&path) else {
            log::warn!("could not read file");
            return Default::default();
        };

        let config = match toml::from_str(&contents) {
            Ok(x) => x,
            Err(e) => {
                log::warn!("error deserializing config: {e}");
                return Default::default();
            }
        };

        log::debug!("config: {config:?}");

        config
    }

    pub fn get_account_config(&self, quaddle_url: &Url, user: UserId) -> Option<&Account> {
        Some(self.accounts.get(quaddle_url)?.get(&user)?)
    }

    pub fn channel_at(
        &self,
        gateway_state: &GatewayState,
        server: &Url,
        idx: usize,
    ) -> Option<&Channel> {
        self.get_account_config(server, gateway_state.user()?.id)?
            .channels
            .get(idx)
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        let Some(dirs) = BaseDirs::new() else {
            log::warn!("could not find basedirs");
            return;
        };

        let path = dirs.config_dir().join(CONFIG_PATH);

        let toml_str = match toml::to_string_pretty(&self) {
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
            return;
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    pub channels: Vec<Channel>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
}

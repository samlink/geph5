use geph5_broker_protocol::Credential;
use geph5_client::Config;
use once_cell::sync::Lazy;

use crate::store_cell::StoreCell;

pub static DEFAULT_SETTINGS: Lazy<serde_yaml::Value> = Lazy::new(|| {
    serde_yaml::from_str(include_str!("client.yaml")).unwrap()
});

pub fn get_config() -> anyhow::Result<Config> {
    let yaml: serde_yaml::Value = DEFAULT_SETTINGS.to_owned();
    let json: serde_json::Value = serde_json::to_value(&yaml)?;
    let mut cfg: Config = serde_json::from_value(json)?;
    cfg.credentials = Credential::LegacyUsernamePassword { username: USERNAME.get(), password: PASSWORD.get() };
    cfg.vpn = VPN_MODE.get();
    cfg.passthrough_china = true;
    Ok(cfg)
}

pub static USERNAME: Lazy<StoreCell<String>> =
    Lazy::new(|| StoreCell::new_persistent("username", || "".to_string()));

pub static PASSWORD: Lazy<StoreCell<String>> =
    Lazy::new(|| StoreCell::new_persistent("password", || "".to_string()));

pub static VPN_MODE: Lazy<StoreCell<bool>> =
    Lazy::new(|| StoreCell::new_persistent("vpn_mode", || false));

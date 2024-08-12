use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::VerifyingKey;
use isocountry::CountryCode;
use language_tags::LanguageTag;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// This fully describes a particular exit.
pub struct ExitDescriptor {
    /// The listening port for the client-to-exit protocol
    pub c2e_listen: SocketAddr,
    /// The listening port for the bridge-to-exit protocol
    pub b2e_listen: SocketAddr,
    /// The country code
    pub country: CountryCode,
    /// The city code
    pub city: String,
    /// How loaded the server is
    pub load: f32,
    /// When does this descriptor expire?
    pub expiry: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// This fully describes all the available exits in the system.
pub struct ExitList {
    /// All the exits in the system
    pub all_exits: Vec<(VerifyingKey, ExitDescriptor)>,
    pub city_names: HashMap<String, HashMap<LanguageTag, String>>,
}

impl ExitList {
    /// A convenience method to find the overall expiry time of the exit list.
    pub fn expiry(&self) -> SystemTime {
        UNIX_EPOCH
            + Duration::from_secs(
                self.all_exits
                    .iter()
                    .map(|exit| exit.1.expiry)
                    .min()
                    .unwrap_or_default(),
            )
    }
}

use std::{
    convert::Infallible,
    sync::LazyLock,
    time::{Duration, SystemTime},
};

use anyctx::AnyCtx;
use async_trait::async_trait;
use geph5_broker_protocol::{
    puzzle::solve_puzzle, AccountLevel, ExitDescriptor, NewsItem, VoucherInfo,
};

use itertools::Itertools;
use nanorpc::{nanorpc_derive, JrpcRequest, JrpcResponse, RpcService, RpcTransport};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use slab::Slab;

use crate::{
    broker_client, client::CtxField, logging::get_json_logs, stats::stat_get_num,
    traffcount::TRAFF_COUNT, updates::get_update_manifest, Config,
};

#[nanorpc_derive]
#[async_trait]
pub trait ControlProtocol {
    async fn conn_info(&self) -> ConnInfo;
    async fn stat_num(&self, stat: String) -> f64;
    async fn start_time(&self) -> SystemTime;
    async fn stop(&self);

    async fn recent_logs(&self) -> Vec<String>;

    // broker-proxying stuff

    async fn check_secret(&self, secret: String) -> Result<bool, String>;
    async fn user_info(&self, secret: String) -> Result<UserInfo, String>;
    async fn start_registration(&self) -> Result<usize, String>;
    async fn poll_registration(&self, idx: usize) -> Result<RegistrationProgress, String>;
    async fn convert_legacy_account(
        &self,
        username: String,
        password: String,
    ) -> Result<String, String>;
    async fn stat_history(&self, stat: String) -> Result<Vec<f64>, String>;
    async fn exit_list(&self) -> Result<Vec<ExitDescriptor>, String>;
    async fn latest_news(&self, lang: String) -> Result<Vec<NewsItem>, String>;
    async fn price_points(&self) -> Result<Vec<(u32, f64)>, String>;
    async fn create_payment(
        &self,
        secret: String,
        days: u32,
        method: String,
    ) -> Result<String, String>;
    async fn get_free_voucher(&self, secret: String) -> Result<Option<VoucherInfo>, String>;
    async fn redeem_voucher(&self, secret: String, code: String) -> Result<i32, String>;
    async fn export_debug_pack(
        &self,
        email: Option<String>,
        contents: String,
    ) -> Result<(), String>;

    async fn get_update_manifest(&self) -> Result<(serde_json::Value, String), String>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "state")]
pub enum ConnInfo {
    Disconnected,
    Connecting,
    Connected(ConnectedInfo),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectedInfo {
    pub protocol: String,
    pub bridge: String,

    pub exit: ExitDescriptor,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub level: AccountLevel,
    pub expiry: Option<u64>,
}

pub struct ControlProtocolImpl {
    pub ctx: AnyCtx<Config>,
}

pub static CURRENT_CONN_INFO: CtxField<Mutex<ConnInfo>> = |_| Mutex::new(ConnInfo::Disconnected);

static REGISTRATIONS: LazyLock<Mutex<Slab<RegistrationProgress>>> =
    LazyLock::new(|| Mutex::new(Slab::new()));

#[derive(Serialize, Deserialize, Clone)]
pub struct RegistrationProgress {
    pub progress: f64,
    pub secret: Option<String>,
}

#[async_trait]
impl ControlProtocol for ControlProtocolImpl {
    async fn conn_info(&self) -> ConnInfo {
        self.ctx.get(CURRENT_CONN_INFO).lock().clone()
    }

    async fn stat_num(&self, stat: String) -> f64 {
        stat_get_num(&self.ctx, &stat)
    }

    async fn start_time(&self) -> SystemTime {
        static START_TIME: CtxField<SystemTime> = |_| SystemTime::now();
        *self.ctx.get(START_TIME)
    }

    async fn stop(&self) {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            std::process::exit(0);
        });
    }

    async fn recent_logs(&self) -> Vec<String> {
        get_json_logs().split("\n").map(|s| s.to_string()).collect()
    }

    async fn check_secret(&self, secret: String) -> Result<bool, String> {
        let res = broker_client(&self.ctx)
            .map_err(|e| format!("{:?}", e))?
            .get_user_info_by_cred(geph5_broker_protocol::Credential::Secret(secret))
            .await
            .map_err(|e| format!("{:?}", e))?
            .map_err(|e| format!("{:?}", e))?;
        Ok(res.is_some())
    }

    async fn user_info(&self, secret: String) -> Result<UserInfo, String> {
        let res = broker_client(&self.ctx)
            .map_err(|e| format!("{:?}", e))?
            .get_user_info_by_cred(geph5_broker_protocol::Credential::Secret(secret))
            .await
            .map_err(|e| format!("{:?}", e))?
            .map_err(|e| format!("{:?}", e))?
            .ok_or_else(|| "no such user".to_string())?;
        Ok(UserInfo {
            level: if res.plus_expires_unix.is_some() {
                AccountLevel::Plus
            } else {
                AccountLevel::Free
            },
            expiry: res.plus_expires_unix,
        })
    }

    async fn start_registration(&self) -> Result<usize, String> {
        let (puzzle, difficulty) = broker_client(&self.ctx)
            .map_err(|e| format!("{:?}", e))?
            .get_puzzle()
            .await
            .map_err(|e| format!("{:?}", e))?;
        tracing::debug!(puzzle, difficulty, "got puzzle");
        let idx = REGISTRATIONS.lock().insert(RegistrationProgress {
            progress: 0.0,
            secret: None,
        });
        let ctx = self.ctx.clone();
        smolscale::spawn(async move {
            loop {
                let fallible = async {
                    let solution = {
                        let puzzle = puzzle.clone();
                        smol::unblock(move || {
                            solve_puzzle(&puzzle, difficulty, |progress| {
                                REGISTRATIONS.lock()[idx] = RegistrationProgress {
                                    progress,
                                    secret: None,
                                }
                            })
                        })
                        .await
                    };
                    let secret = broker_client(&ctx)?
                        .register_user_secret(puzzle.clone(), solution)
                        .await?
                        .map_err(|e| anyhow::anyhow!(e))?;
                    REGISTRATIONS.lock()[idx] = RegistrationProgress {
                        progress: 1.0,
                        secret: Some(secret.clone()),
                    };
                    anyhow::Ok(secret)
                };
                if let Err(err) = fallible.await {
                    tracing::warn!(err = debug(err), "restarting registration")
                } else {
                    break;
                }
            }
        })
        .detach();
        Ok(idx)
    }

    async fn poll_registration(&self, idx: usize) -> Result<RegistrationProgress, String> {
        tracing::debug!(idx, "polling registration");
        let registers = REGISTRATIONS.lock();
        registers
            .get(idx)
            .cloned()
            .ok_or_else(|| "no such registration".to_string())
    }

    async fn convert_legacy_account(
        &self,
        username: String,
        password: String,
    ) -> Result<String, String> {
        Ok(broker_client(&self.ctx)
            .map_err(|e| format!("{:?}", e))?
            .upgrade_to_secret(geph5_broker_protocol::Credential::LegacyUsernamePassword {
                username,
                password,
            })
            .await
            .map_err(|e| format!("{:?}", e))?
            .map_err(|e| format!("{:?}", e))?)
    }

    async fn stat_history(&self, stat: String) -> Result<Vec<f64>, String> {
        if stat != "traffic" {
            return Err(format!("bad: {stat}"));
        }
        Ok(self.ctx.get(TRAFF_COUNT).read().unwrap().speed_history())
    }

    async fn exit_list(&self) -> Result<Vec<ExitDescriptor>, String> {
        let resp = broker_client(&self.ctx)
            .map_err(|e| format!("{:?}", e))?
            .get_exits()
            .await
            .map_err(|e| format!("{:?}", e))?
            .map_err(|e| format!("{:?}", e))?;
        Ok(resp.inner.all_exits.iter().map(|s| s.1.clone()).collect())
    }

    async fn latest_news(&self, lang: String) -> Result<Vec<NewsItem>, String> {
        let client = broker_client(&self.ctx).map_err(|e| format!("{:?}", e))?;
        Ok(client
            .get_news(lang)
            .await
            .map_err(|s| s.to_string())?
            .map_err(|s| s.to_string())?)
    }

    async fn get_free_voucher(&self, secret: String) -> Result<Option<VoucherInfo>, String> {
        let client = broker_client(&self.ctx).map_err(|e| format!("{:?}", e))?;
        Ok(client
            .get_free_voucher(secret)
            .await
            .map_err(|s| s.to_string())?
            .map_err(|s| s.to_string())?)
    }

    async fn redeem_voucher(&self, secret: String, code: String) -> Result<i32, String> {
        let client = broker_client(&self.ctx).map_err(|e| format!("{:?}", e))?;

        // Call the broker's redeem_voucher method directly with the secret
        client
            .redeem_voucher(secret, code)
            .await
            .map_err(|s| s.to_string())?
            .map_err(|s| s.to_string())
    }

    async fn price_points(&self) -> Result<Vec<(u32, f64)>, String> {
        let client = broker_client(&self.ctx).map_err(|e| format!("{:?}", e))?;
        Ok(client
            .raw_price_points()
            .await
            .map_err(|s| s.to_string())?
            .map_err(|s| s.to_string())?
            .into_iter()
            .map(|(a, b)| (a, b as f64 / 100.0))
            .collect())
    }

    async fn create_payment(
        &self,
        secret: String,
        days: u32,
        method: String,
    ) -> Result<String, String> {
        let client = broker_client(&self.ctx).map_err(|e| format!("{:?}", e))?;
        Ok(client
            .create_payment(secret, days, method)
            .await
            .map_err(|s| s.to_string())?
            .map_err(|s| s.to_string())?)
    }

    async fn export_debug_pack(
        &self,
        email: Option<String>,
        contents: String,
    ) -> Result<(), String> {
        let client = broker_client(&self.ctx).map_err(|e| format!("{:?}", e))?;
        client
            .upload_debug_pack(email, contents)
            .await
            .map_err(|s| s.to_string())?
            .map_err(|s| s.to_string())?;
        Ok(())
    }

    async fn get_update_manifest(&self) -> Result<(serde_json::Value, String), String> {
        get_update_manifest().await.map_err(|e| format!("{:?}", e))
    }
}

pub struct DummyControlProtocolTransport(pub ControlService<ControlProtocolImpl>);

#[async_trait]
impl RpcTransport for DummyControlProtocolTransport {
    type Error = Infallible;

    async fn call_raw(&self, req: JrpcRequest) -> Result<JrpcResponse, Self::Error> {
        Ok(self.0.respond_raw(req).await)
    }
}

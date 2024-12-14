mod asn_count;
mod listen_forward;

use std::{
    i32,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::{Duration, SystemTime},
};

use anyhow::Context as _;
use asn_count::ASN_BYTES;
use geph5_broker_protocol::{BridgeDescriptor, Mac};
use listen_forward::{listen_forward_loop, BYTE_COUNT};
use rand::Rng;
use sillad::{
    dialer::DialerExt,
    tcp::{TcpDialer, TcpListener},
};
use sillad_sosistab3::{listener::SosistabListener, Cookie};
use smol::future::FutureExt as _;

use smol_timeout2::TimeoutExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
        .with(
            EnvFilter::builder()
                .with_default_directive("geph5_bridge".parse().unwrap())
                .from_env_lossy(),
        )
        .init();
    smolscale::block_on(async {
        let my_ip = IpAddr::from_str(
            String::from_utf8_lossy(
                &reqwest::get("https://checkip.amazonaws.com/")
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap(),
            )
            .trim(),
        )
        .unwrap();

        let port = rand::thread_rng().gen_range(1024..10000);
        let control_listen = SocketAddr::new(my_ip, port);
        let control_cookie = format!("bridge-cookie-{}", rand::random::<u128>());

        let upload_loop = broker_loop(control_listen, control_cookie.clone());
        let listen_loop = async {
            loop {
                let listener = TcpListener::bind(format!("0.0.0.0:{port}").parse().unwrap())
                    .await
                    .unwrap();

                let control_listener =
                    SosistabListener::new(listener, Cookie::new(&control_cookie));
                if let Err(err) = listen_forward_loop(my_ip, control_listener).await {
                    tracing::error!(err = %err, "error in listen_forward_loop");
                }
                smol::Timer::after(Duration::from_secs(1)).await;
            }
        };
        upload_loop.race(listen_loop).await
    })
}

async fn broker_loop(control_listen: SocketAddr, control_cookie: String) {
    let auth_token = std::env::var("GEPH5_BRIDGE_TOKEN").unwrap();
    let pool = std::env::var("GEPH5_BRIDGE_POOL").unwrap();
    let broker_addr: SocketAddr = std::env::var("GEPH5_BROKER_ADDR").unwrap().parse().unwrap();
    tracing::info!(
        auth_token,
        broker_addr = display(broker_addr),
        "starting upload loop"
    );

    let bridge_key = format!("bridges.{pool}");

    let broker_rpc = geph5_broker_protocol::BrokerClient(nanorpc_sillad::DialerTransport(
        TcpDialer {
            dest_addr: broker_addr,
        }
        .timeout(Duration::from_secs(1)),
    ));

    let upload_loop = async {
        loop {
            tracing::info!(
                auth_token,
                broker_addr = display(broker_addr),
                "uploading..."
            );

            let res = async {
                broker_rpc
                    .insert_bridge(Mac::new(
                        BridgeDescriptor {
                            control_listen,
                            control_cookie: control_cookie.clone(),
                            pool: pool.clone(),
                            expiry: SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                + 120,
                        },
                        blake3::hash(auth_token.as_bytes()).as_bytes(),
                    ))
                    .timeout(Duration::from_secs(2))
                    .await
                    .context("insert bridge timed out")??
                    .map_err(|e| anyhow::anyhow!(e))?;
                anyhow::Ok(())
            };
            if let Err(err) = res.await {
                tracing::error!(err = %err, "error in upload_loop");
            }
            smol::Timer::after(Duration::from_secs(10)).await;
        }
    };

    let stats_loop = async {
        loop {
            tracing::info!(auth_token, broker_addr = display(broker_addr), "stats...");
            let res = async {
                let byte_count = BYTE_COUNT.swap(0, std::sync::atomic::Ordering::Relaxed);
                broker_rpc
                    .incr_stat(format!("{bridge_key}.byte_count"), byte_count as _)
                    .timeout(Duration::from_secs(2))
                    .await
                    .context("incrementing bytes timed out")??;

                let asn_bytes: Vec<(u32, u64)> = ASN_BYTES
                    .iter()
                    .map(|item| {
                        let asn_byte_count =
                            item.value().swap(0, std::sync::atomic::Ordering::Relaxed);
                        (*item.key(), asn_byte_count)
                    })
                    .collect();
                ASN_BYTES.clear();
                for (asn, bytes) in asn_bytes {
                    let bytes = bytes.min(i32::MAX as u64) as i32;
                    broker_rpc
                        .incr_stat(format!("{bridge_key}.asn.{}", asn), bytes)
                        .timeout(Duration::from_secs(2))
                        .await
                        .context("incrementing ASN timed out")??;
                    tracing::debug!("incremented ASN {} with {} bytes", asn, bytes);
                }
                anyhow::Ok(())
            };
            if let Err(err) = res.await {
                tracing::error!(err = %err, "error in stats_loop");
            }
            smol::Timer::after(Duration::from_secs(3)).await;
        }
    };
    upload_loop.race(stats_loop).await
}

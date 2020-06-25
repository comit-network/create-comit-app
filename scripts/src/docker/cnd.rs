use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
};

use anyhow::Context;
use shiplift::ContainerOptions;

use crate::docker::{
    self, docker_daemon_ip, free_local_port::free_local_port, DockerImage, File, LogMessage,
    DOCKER_NETWORK,
};
use serde::Serializer;

const IMAGE: &str = "comitnetwork/cnd:0.8.0";

#[derive(derive_more::Display, Copy, Clone)]
#[display(fmt = "http://{}:{}", ip, port)]
pub struct HttpEndpoint {
    port: u16,
    ip: Ipv4Addr,
}

pub struct CndInstance {
    pub http_endpoint: HttpEndpoint,
}

pub async fn new_instance(index: u32) -> anyhow::Result<CndInstance> {
    let settings = Settings::default();

    let settings = toml::to_string(&settings).context("failed to serialize settings")?;

    let mut options_builder = ContainerOptions::builder(IMAGE);
    options_builder.network_mode(DOCKER_NETWORK);
    options_builder.name(&format!("cnd_{}", index));
    options_builder.cmd(vec!["--", "cnd", "--config=/cnd.toml"]);

    let http_port = free_local_port().await?;
    options_builder.expose(8080, "tcp", http_port as u32);

    let options = options_builder.build();

    docker::start(
        DockerImage(IMAGE),
        options,
        LogMessage("Starting HTTP server on"),
        vec![File {
            location: Path::new("/cnd.toml"),
            content: settings.as_bytes(),
        }],
    )
    .await?;

    Ok(CndInstance {
        http_endpoint: HttpEndpoint {
            port: http_port,
            ip: docker_daemon_ip()?,
        },
    })
}

#[derive(Clone, Debug, serde::Serialize, Default)]
struct Settings {
    network: Network,
    http_api: HttpApi,
    data: Data,
    logging: Logging,
    bitcoin: Bitcoin,
    ethereum: Ethereum,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Network {
    listen: Vec<String>,
}
#[derive(Clone, Debug, serde::Serialize)]
struct HttpApi {
    socket: Socket,
    cors: Cors,
}

#[derive(Clone, Debug)]
struct Socket {
    address: IpAddr,
    port: u16,
}

impl serde::Serialize for Socket {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}:{}", self.address.to_string(), self.port))
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct Cors {
    allowed_origins: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Data {
    dir: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Logging {
    level: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Bitcoin {
    network: String,
    bitcoind: Bitcoind,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Bitcoind {
    node_url: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Ethereum {
    chain_id: i16,
    geth: Geth,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Geth {
    node_url: String,
}

impl Default for Network {
    fn default() -> Network {
        Network {
            listen: vec!["/ip4/0.0.0.0/tcp/9939".into()],
        }
    }
}

impl Default for HttpApi {
    fn default() -> HttpApi {
        HttpApi {
            socket: Socket::default(),
            cors: Cors::default(),
        }
    }
}

impl Default for Socket {
    fn default() -> Socket {
        Socket {
            address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: 8080,
        }
    }
}

impl Default for Cors {
    fn default() -> Cors {
        Cors {
            allowed_origins: String::from("all"),
        }
    }
}

impl Default for Data {
    fn default() -> Self {
        Data {
            dir: "/home/cnd/.local/share/comit/".to_string(),
        }
    }
}

impl Default for Logging {
    fn default() -> Self {
        Logging {
            level: "Debug".to_string(),
        }
    }
}

impl Default for Bitcoin {
    fn default() -> Self {
        Bitcoin {
            network: "regtest".to_string(),
            bitcoind: Bitcoind {
                node_url: "http://bitcoin:18443".to_string(),
            },
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Ethereum {
            chain_id: 17,
            geth: Geth {
                node_url: "http://ethereum:8545".to_string(),
            },
        }
    }
}

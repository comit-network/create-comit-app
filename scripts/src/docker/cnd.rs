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

const IMAGE: &str = "comitnetwork/cnd:0.4.0";

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
    let settings = Settings {
        bitcoin: Bitcoin {
            network: String::from("regtest"),
            node_url: "http://bitcoin:18443".to_string(),
        },
        ethereum: Ethereum {
            chain_id: 17,
            node_url: "http://ethereum:8545".to_string(),
        },
        ..Default::default()
    };

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

#[derive(Clone, Debug, serde::Serialize)]
struct Socket {
    address: IpAddr,
    port: u16,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Cors {
    allowed_origins: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Logging {
    level: String,
    structured: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Bitcoin {
    network: String,
    node_url: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Ethereum {
    chain_id: i8,
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

impl Default for Logging {
    fn default() -> Self {
        Logging {
            level: "DEBUG".to_string(),
            structured: false,
        }
    }
}

impl Default for Bitcoin {
    fn default() -> Self {
        Bitcoin {
            network: "regtest".to_string(),
            node_url: "http://localhost:18443".to_string(),
        }
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Ethereum {
            chain_id: 17,
            node_url: "http://localhost:8545".to_string(),
        }
    }
}

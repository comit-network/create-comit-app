use crate::{
    docker::{self, DockerImage, LogMessage, DOCKER_NETWORK},
    temp_fs,
};
use futures::compat::Future01CompatExt;
use shiplift::ContainerOptions;
use std::net::{IpAddr, Ipv4Addr};

const IMAGE: &str = "comitnetwork/cnd:0.4.0";

#[derive(derive_more::Display, Copy, Clone)]
#[display(fmt = "http://localhost:{}", port)]
pub struct HttpEndpoint {
    port: u16,
}

pub struct CndInstance {
    pub http_endpoint: HttpEndpoint,
}

pub async fn new_instance(index: u32) -> anyhow::Result<CndInstance> {
    let config_folder = temp_fs::temp_folder()?;

    let settings = Settings {
        bitcoin: Bitcoin {
            network: String::from("regtest"),
            node_url: "http://bitcoin:18443".to_string(),
        },
        ethereum: Ethereum {
            network: String::from("regtest"),
            node_url: "http://ethereum:8545".to_string(),
        },
        ..Default::default()
    };

    let config_file = config_folder.join("cnd.toml");
    let settings = toml::to_string(&settings).expect("could not serialize hardcoded settings");

    tokio::fs::write(config_file, settings).compat().await?;

    let mut options_builder = ContainerOptions::builder(IMAGE);
    options_builder.network_mode(DOCKER_NETWORK);
    options_builder.name(&format!("cnd_{}", index));
    options_builder.cmd(vec!["--", "cnd", "--config=/config/cnd.toml"]);
    options_builder.volumes(vec![&format!("{}:/config", config_folder.display())]);

    let http_port =
        port_check::free_local_port().ok_or(anyhow::anyhow!("failed to grab a free local port"))?;
    options_builder.expose(8080, "tcp", http_port as u32);

    let options = options_builder.build();

    docker::start(
        DockerImage(IMAGE),
        options,
        LogMessage("Starting HTTP server on"),
    )
    .await?;

    Ok(CndInstance {
        http_endpoint: HttpEndpoint { port: http_port },
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
    network: String,
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
            network: "regtest".to_string(),
            node_url: "http://localhost:8545".to_string(),
        }
    }
}

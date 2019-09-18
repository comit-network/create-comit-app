use rand;
use rand::Rng;
use serde::Serialize;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::process::Command;
use tempfile::{self, TempPath};
use tokio_process::{Child, CommandExt};

pub struct Cnd {
    pub settings: Settings,
    _config_file: TempPath,
    pub process: Child,
}

impl Cnd {
    pub fn start(settings: Settings) -> Cnd {
        let mut config_file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        config_file
            .write(
                toml::to_string(&settings)
                    .expect("could not serialize settings")
                    .as_ref(),
            )
            .expect("could not write to temporary file");
        let config_file = config_file.into_temp_path();

        let process = Command::new("cnd")
            .arg("--config")
            .arg(config_file.to_str().unwrap())
            .stdout(std::process::Stdio::null())
            .spawn_async()
            .expect("failed to start btsieve");

        Cnd {
            settings,
            _config_file: config_file,
            process,
        }
    }
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Settings {
    pub comit: Comit,
    pub network: Network,
    pub http_api: HttpSocket,
    pub btsieve: Btsieve,
    pub web_gui: Option<HttpSocket>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Comit {
    #[serde(with = "hex_serde")]
    pub secret_seed: [u8; 32],
}

#[derive(Clone, Debug, Serialize)]
pub struct Network {
    pub listen: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct HttpSocket {
    pub address: IpAddr,
    pub port: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct Btsieve {
    pub url: String,
    pub bitcoin: PollParameters,
    pub ethereum: PollParameters,
}

#[derive(Clone, Debug, Serialize)]
pub struct PollParameters {
    pub poll_interval_secs: u16,
    pub network: String,
}

impl Default for Comit {
    fn default() -> Comit {
        let mut secret_seed = [0u8; 32];
        rand::thread_rng().fill(&mut secret_seed);

        Comit { secret_seed }
    }
}

impl Default for Network {
    fn default() -> Network {
        Network {
            listen: vec![String::from("/ip4/0.0.0.0/tcp/9939")],
        }
    }
}

impl Default for HttpSocket {
    fn default() -> HttpSocket {
        HttpSocket {
            address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: 8000,
        }
    }
}

impl Default for Btsieve {
    fn default() -> Btsieve {
        Btsieve {
            url: String::from("http://localhost:8181"),
            bitcoin: PollParameters {
                poll_interval_secs: 300,
                network: String::from("regtest"),
            },
            ethereum: PollParameters {
                poll_interval_secs: 20,
                network: String::from("regtest"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::Future;
    use ureq;

    #[test]
    fn can_ping_cnd() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let port = port_check::free_local_port().unwrap().into();
        let settings = Settings {
            http_api: HttpSocket {
                port,
                ..Default::default()
            },
            ..Default::default()
        };

        let cnd = Cnd::start(settings);

        runtime.spawn(cnd.process.map(|_| ()).map_err(|_| ()));

        std::thread::sleep(std::time::Duration::from_millis(5000));

        let endpoint = format!("http://localhost:{}", port);
        assert!(ureq::get(&endpoint).call().ok())
    }
}

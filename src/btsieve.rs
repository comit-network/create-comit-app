use serde::Serialize;
use std::io::Write;
use std::net::IpAddr;
use std::process::Command;
use std::str::FromStr;
use tempfile::{self, TempPath};
use tokio_process::{Child, CommandExt};

pub struct Btsieve {
    pub settings: Settings,
    _config_file: TempPath,
    pub process: Child,
}

impl Btsieve {
    pub fn start(settings: Settings) -> Btsieve {
        let mut config_file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        config_file
            .write(
                toml::to_string(&settings)
                    .expect("could not serialize settings")
                    .as_ref(),
            )
            .expect("could not write to temporary file");
        let config_file = config_file.into_temp_path();

        let process = Command::new("btsieve")
            .arg("--config")
            .arg(config_file.to_str().unwrap())
            .stdout(std::process::Stdio::null())
            .spawn_async()
            .expect("failed to start btsieve");

        Btsieve {
            settings,
            _config_file: config_file,
            process,
        }
    }
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct Settings {
    pub log_levels: LogLevels,
    pub http_api: HttpApi,
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogLevels {
    pub btsieve: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct HttpApi {
    pub address_bind: IpAddr,
    pub port_bind: u16,
}

#[derive(Debug, Serialize, Clone)]
pub struct Bitcoin {
    pub network: String,
    pub node_url: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Ethereum {
    pub node_url: String,
}

impl Default for LogLevels {
    fn default() -> LogLevels {
        LogLevels {
            btsieve: "DEBUG".to_string(),
        }
    }
}

impl Default for HttpApi {
    fn default() -> HttpApi {
        HttpApi {
            address_bind: IpAddr::from_str("0.0.0.0").expect("can't parse IpAddr from &str"),
            port_bind: 8181,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::Future;
    use ureq;

    #[test]
    fn can_ping_btsieve() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let port_bind = port_check::free_local_port().unwrap().into();
        let settings = Settings {
            http_api: HttpApi {
                port_bind,
                ..Default::default()
            },
            ..Default::default()
        };
        let btsieve = Btsieve::start(settings);

        runtime.spawn(btsieve.process.map(|_| ()).map_err(|_| ()));

        std::thread::sleep(std::time::Duration::from_millis(5000));

        let endpoint = format!("http://localhost:{}/health", port_bind);
        assert!(ureq::get(&endpoint)
            .set("Expected-Version", "0.2.0")
            .call()
            .ok())
    }
}

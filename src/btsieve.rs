use envfile::EnvFile;
use futures::Future;
use serde::Serialize;
use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;
use tempfile;
use tokio_process::CommandExt;

pub const HTTP_PORT: &str = "BTSIEVE_HTTP_PORT";

pub struct Btsieve {
    pub settings: Settings,
}

impl Btsieve {
    pub fn start(settings: Settings, envfile_path: PathBuf) -> impl Future<Item = (), Error = ()> {
        let mut envfile = EnvFile::new(&envfile_path).unwrap();
        envfile
            .update(HTTP_PORT, &settings.http_api.port_bind.to_string())
            .write()
            .unwrap();

        let mut config_file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        config_file
            .write(
                toml::to_string(&settings)
                    .expect("could not serialize settings")
                    .as_ref(),
            )
            .expect("could not write to temporary file");
        let config_file = config_file.into_temp_path();

        let child = Command::new("btsieve")
            .arg("--config")
            .stdout(std::process::Stdio::null())
            .arg(config_file.to_str().unwrap())
            .spawn_async();

        // FIXME: Should wait until btsieve logs
        // "warp drive engaged: listening on http://0.0.0.0:8181" instead
        sleep(Duration::from_millis(1000));

        let future = child
            .expect("failed to start btsieve")
            .map(|status| println!("exit status: {}", status))
            .map_err(|e| panic!("failed to wait for exit: {}", e));

        future
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
        let file = tempfile::Builder::new().tempfile().unwrap();

        runtime.spawn(Btsieve::start(settings, file.path().to_path_buf()));

        let endpoint = format!("http://localhost:{}/health", port_bind);
        assert!(ureq::get(&endpoint)
            .set("Expected-Version", "0.2.0")
            .call()
            .ok())
    }
}

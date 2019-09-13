use futures::Future;
use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use tempfile;
use tokio::runtime::Runtime;
use tokio_process::CommandExt;

pub struct Cnd;

impl Cnd {
    pub fn start(port: u32) -> impl Future<Item = (), Error = ()> {
        // TODO: Use TOML library to have a struct instead
        let config = format!(
            r#"
[comit]
secret_seed = "4481d31defc255c088891b6fb778968c5b813a8d791aec1b4d06f92cb08f4664"

[network]
listen = ["/ip4/0.0.0.0/tcp/9939"]

[http_api]
address = "0.0.0.0"
port = {}

[btsieve]
url = "http://localhost:8181/"

[btsieve.bitcoin]
poll_interval_secs = 300
network = "regtest"

[btsieve.ethereum]
poll_interval_secs = 20
network = "regtest""#,
            port
        );

        let mut config_file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(config_file, "{}", config).unwrap();

        let config_file = config_file.into_temp_path();

        let child = Command::new("cnd")
            .arg("--config")
            .arg(config_file.to_str().unwrap())
            .spawn_async();

        let future = child
            .expect("failed to start cnd")
            .map(|status| println!("exit status: {}", status))
            .map_err(|e| panic!("failed to wait for exit: {}", e));

        // FIXME: Should wait until cnd logs "Starting HTTP server on V4(0.0.0.0:8000)" instead
        sleep(Duration::from_millis(1000));
        future
    }
}

pub fn start_cnd() {}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest;

    #[test]
    fn can_ping_cnd() {
        let port = 8000;
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let future = Cnd::start(port);

        runtime.spawn(future);

        let endpoint = format!("http://localhost:{}", port);
        assert!(reqwest::get(&endpoint).unwrap().status().is_success())
    }
}

use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use tempfile;

pub struct Btsieve {
    pub port: u32,
}

impl Btsieve {
    pub fn start() -> Btsieve {
        let port = 8181;

        let config = format!(
            r#"
[http_api]
address_bind="0.0.0.0"
port_bind={}

[log_levels]
btsieve="DEBUG""#,
            port
        );

        let mut config_file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(config_file, "{}", config).unwrap();

        let config_file = config_file.into_temp_path();

        Command::new("btsieve")
            .arg("--config")
            .stdout(std::process::Stdio::null())
            .arg(config_file.to_str().unwrap())
            .spawn()
            .unwrap();

        // FIXME: Should wait until btsieve logs
        // "warp drive engaged: listening on http://0.0.0.0:8181" instead
        sleep(Duration::from_millis(1000));

        Btsieve { port }
    }
}

pub fn start_btsieve() {}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest;

    #[test]
    fn can_ping_btsieve() {
        let btsieve = Btsieve::start();

        let endpoint = format!("http://localhost:{}/health", btsieve.port);
        assert!(reqwest::get(&endpoint).unwrap().status().is_success())
    }
}

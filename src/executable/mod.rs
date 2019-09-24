use futures::Future;
use serde::Serialize;
use std::io::Write;
use std::process::Command;
use tempfile;
use tempfile::TempPath;
use tokio_process::CommandExt;

pub mod btsieve;
pub mod cnd;

pub trait Program {
    const COMMAND: &'static str;
    const LOG_READY: &'static str;
}

// config_file is only here to ensure it is not erased (when dropped) before the executable fully runs
#[allow(dead_code)]
pub struct Executable {
    config_file: TempPath,
    pub future: Box<dyn Future<Item = (), Error = ()> + Send>,
}

impl Executable where {
    pub fn start<P: Program, S: Serialize>(settings: S) -> Self {
        let mut config_file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        config_file
            .write_all(
                toml::to_string(&settings)
                    .expect("could not serialize settings")
                    .as_ref(),
            )
            .expect("could not write to temporary file");
        let config_file = config_file.into_temp_path();

        let child = Command::new(P::COMMAND)
            .stdout(std::process::Stdio::null())
            .arg("--config")
            .arg(config_file.to_str().unwrap())
            .spawn_async();
        let future = child
            .expect("failed to start executable")
            .map(move |status| println!("{}'s exit status: {}", P::COMMAND, status))
            .map_err(|e| panic!("failed to wait for exit: {}", e));

        Executable {
            config_file,
            future: Box::new(future),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executable::cnd::Cnd;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn can_start_cnd() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let settings = cnd::Settings::default();
        let port = settings.http_api.port;

        let cnd = Executable::start::<Cnd, _>(settings);

        runtime.spawn(cnd.future);

        // FIXME: Should wait until cnd logs "Starting HTTP server on V4(0.0.0.0:8000)" instead
        sleep(Duration::from_millis(1000));

        let endpoint = format!("http://localhost:{}", port);

        let response = ureq::get(&endpoint).call();
        println!("{:?}", response);
        println!("{:?}", response.into_string());

        assert!(ureq::get(&endpoint).call().ok())
    }
}

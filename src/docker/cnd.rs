use crate::docker::{ExposedPorts, Image};

const HTTP_PORT_CND: &str = "HTTP_PORT_CND";

pub struct Cnd;

impl Image for Cnd {
    const IMAGE: &'static str = "comitnetwork/cnd:0.2.1";
    const LOG_READY: &'static str = "Starting HTTP server on";

    fn arguments_for_create() -> Vec<&'static str> {
        vec!["--", "cnd", "--config=/config/cnd.toml"]
    }

    fn expose_ports() -> Vec<ExposedPorts> {
        vec![ExposedPorts {
            for_client: true,
            srcport: 8080,
            env_file_key: HTTP_PORT_CND.to_string(),
            env_file_value: Box::new(|port| format!("http://localhost:{}", port)),
        }]
    }

    fn new(_: Option<String>) -> Self {
        Self
    }
    fn post_start_actions(&self) {}
}

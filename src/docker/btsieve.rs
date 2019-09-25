use crate::docker::{ExposedPorts, Image};

pub struct Btsieve;

impl Image for Btsieve {
    const IMAGE: &'static str = "comitnetwork/btsieve:0.2.1";
    const LOG_READY: &'static str = "warp drive engaged:";

    fn arguments_for_create() -> Vec<&'static str> {
        vec!["--", "btsieve", "--config=/config/btsieve.toml"]
    }

    fn expose_ports() -> Vec<ExposedPorts> {
        vec![]
    }

    fn new(_: Option<String>) -> Self {
        Self
    }

    fn post_start_actions(&self) {}
}

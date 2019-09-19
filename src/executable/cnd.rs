use rand;
use rand::Rng;
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr};

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
        let port = port_check::free_local_port().expect("Could not find a free port");
        Network {
            listen: vec![format!("/ip4/0.0.0.0/tcp/{}", port)],
        }
    }
}

impl Default for HttpSocket {
    fn default() -> HttpSocket {
        let port = port_check::free_local_port().expect("Could not find a free port");
        HttpSocket {
            address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port,
        }
    }
}

impl Default for Btsieve {
    fn default() -> Btsieve {
        Btsieve {
            url: String::from("http://localhost:8181"),
            bitcoin: PollParameters {
                poll_interval_secs: 1,
                network: String::from("regtest"),
            },
            ethereum: PollParameters {
                poll_interval_secs: 1,
                network: String::from("regtest"),
            },
        }
    }
}
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr};

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
            address_bind: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port_bind: 8080,
        }
    }
}

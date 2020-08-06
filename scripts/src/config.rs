use clarity::Address as EthereumAddress;
use rust_bitcoin::Address as BitcoinAddress;
use serde_derive::Deserialize;
use std::path::Path;

pub const FILE_NAME: &str = "ComitScripts.toml";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub bitcoin: Option<Bitcoin>,
    pub ethereum: Option<Ethereum>,
}

impl Config {
    pub fn from_file(path: &Path) -> anyhow::Result<Config> {
        let toml_string = std::fs::read_to_string(path)?;
        Ok(toml::from_str(toml_string.as_str())?)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bitcoin {
    pub addresses_to_fund: Vec<BitcoinAddress>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ethereum {
    pub addresses_to_fund: Vec<EthereumAddress>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialize() {
        let toml = r#"
          [bitcoin]
          addresses_to_fund = ["1GiYYLU6amEX5NnCeS1fuPG5WcVZfRmbiV"]

          [ethereum]
          addresses_to_fund = ["0x89205A3A3b2A69De6Dbf7f01ED13B2108B2c43e7"]
        "#;

        let _config: Config = toml::from_str(toml).expect("failed to deserialise config toml");
    }
}

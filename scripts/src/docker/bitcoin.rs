use std::net::Ipv4Addr;

use anyhow::Context;
use rust_bitcoin::{
    self,
    hashes::sha256d,
    util::bip32::{ChildNumber, ExtendedPrivKey},
    Address, Amount, Network,
};
use secp256k1::{
    rand::{thread_rng, Rng},
    Secp256k1,
};
use shiplift::ContainerOptions;

use crate::{
    config,
    docker::{
        self, docker_daemon_ip, free_local_port::free_local_port, DockerImage, LogMessage,
        DOCKER_NETWORK,
    },
};
use serde::export::Formatter;
use std::fmt::{self, Display};

const IMAGE: &str = "coblox/bitcoin-core:0.20.0";

pub const USERNAME: &str = "bitcoin";
pub const PASSWORD: &str = "t68ej4UX2pB0cLlGwSwHFBLKxXYgomkXyFyxuBmm2U8=";

const HTTP_PORT: u16 = 18443;

#[derive(derive_more::Display, Copy, Clone)]
#[display(fmt = "{}:{}", ip, port)]
pub struct BitcoindP2PUri {
    port: u16,
    ip: Ipv4Addr,
}

#[derive(derive_more::Display, Copy, Clone)]
#[display(fmt = "http://{}:{}", ip, port)]
pub struct BitcoindHttpEndpoint {
    port: u16,
    ip: Ipv4Addr,
}

pub struct BitcoindInstance {
    pub p2p_uri: BitcoindP2PUri,
    pub http_endpoint: BitcoindHttpEndpoint,
    pub account_0: Account,
    pub account_1: Account,
}

pub async fn new_bitcoind_instance(
    config: Option<config::Bitcoin>,
) -> anyhow::Result<BitcoindInstance> {
    let mut options_builder = ContainerOptions::builder(IMAGE);
    options_builder.name("bitcoin");
    options_builder.network_mode(DOCKER_NETWORK);
    options_builder.cmd(vec![
        "-regtest",
        "-server",
        "-rest",
        "-printtoconsole",
        "-bind=0.0.0.0:18444",
        "-rpcbind=0.0.0.0:18443",
        "-rpcauth=bitcoin:1c0e8f3de84926c04115e7da7e501346$a48f42ad32741dd1755649c8b98663b3ccbebeb75f196389f9a5c8a96b72edb3",
        "-rpcallowip=0.0.0.0/0",
        "-debug=1",
        "-acceptnonstdtxn=0",
        "-txindex",
        "-fallbackfee=0.0002",
    ]);

    let p2p_port = free_local_port().await?;
    options_builder.expose(18444, "tcp", p2p_port as u32);

    let p2p_uri = BitcoindP2PUri {
        port: p2p_port,
        ip: docker_daemon_ip()?,
    };
    options_builder.expose(HTTP_PORT as u32, "tcp", HTTP_PORT as u32);

    let http_endpoint = BitcoindHttpEndpoint {
        port: HTTP_PORT,
        ip: docker_daemon_ip()?,
    };

    let options = options_builder.build();

    docker::start(
        DockerImage(IMAGE),
        options,
        LogMessage("Flushed wallet.dat"),
        vec![],
    )
    .await
    .context("unable to start bitcoind docker image")?;

    let account_0 = fund_new_account(http_endpoint)
        .await
        .context("failed to fund first account")?;
    let account_1 = fund_new_account(http_endpoint)
        .await
        .context("failed to fund second account")?;

    if let Some(config) = config {
        for address in config.addresses_to_fund {
            fund_address(http_endpoint, address).await?;
        }
    }

    Ok(BitcoindInstance {
        p2p_uri,
        http_endpoint,
        account_0,
        account_1,
    })
}

async fn fund_new_account(endpoint: BitcoindHttpEndpoint) -> anyhow::Result<Account> {
    let account = Account::new_random()?;

    let (_, address) = account.first_account();

    fund(
        &endpoint.to_string(),
        address,
        Amount::from_sat(1_000_000_000),
    )
    .await?;

    Ok(account)
}

async fn fund_address(endpoint: BitcoindHttpEndpoint, address: Address) -> anyhow::Result<()> {
    fund(
        &endpoint.to_string(),
        address,
        Amount::from_sat(1_000_000_000),
    )
    .await?;

    Ok(())
}

pub async fn mine_a_block(endpoint: BitcoindHttpEndpoint) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let new_address = new_address(&endpoint.to_string()).await?;

    let _ = client
        .post(&endpoint.to_string())
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&GenerateToAddressRequest::new(1, new_address))
        .send()
        .await?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct DerivationPath(Vec<ChildNumber>);

impl Display for DerivationPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for i in &self.0 {
            write!(f, "/")?;
            fmt::Display::fmt(i, f)?;
        }

        Ok(())
    }
}

impl DerivationPath {
    pub fn bip44_bitcoin_testnet() -> anyhow::Result<Self> {
        Ok(Self(vec![
            ChildNumber::from_hardened_idx(44)?,
            ChildNumber::from_hardened_idx(1)?,
            ChildNumber::from_hardened_idx(0)?,
            ChildNumber::from_normal_idx(0)?,
            ChildNumber::from_normal_idx(0)?,
        ]))
    }
}

#[derive(Debug, Clone)]
pub struct Account {
    pub master: ExtendedPrivKey,
    first_account: rust_bitcoin::util::key::PrivateKey,
    derivation_path: DerivationPath,
}

impl Account {
    fn new_random() -> anyhow::Result<Self> {
        let mut seed = [0u8; 32];
        thread_rng().fill_bytes(&mut seed);

        let master = ExtendedPrivKey::new_master(rust_bitcoin::Network::Regtest, &seed)
            .context("failed to generate new random extended private key from seed")?;

        Account::new(master)
    }

    fn new(master: ExtendedPrivKey) -> anyhow::Result<Self> {
        // define derivation path to derive private keys from the master key
        let derivation_path =
            DerivationPath::bip44_bitcoin_testnet().context("failed to create derivation path")?;

        // derive a private key from the master key
        let priv_key = master
            .derive_priv(&Secp256k1::new(), &derivation_path.0)
            .map(|secret_key| secret_key.private_key)?;

        // it is not great to store derived data in here but since the derivation can fail, it is better to fail early instead of later
        Ok(Self {
            master,
            first_account: priv_key,
            derivation_path,
        })
    }

    fn first_account(&self) -> (rust_bitcoin::util::key::PrivateKey, rust_bitcoin::Address) {
        // derive an address from the private key
        let address = derive_address(self.first_account.key);

        (self.first_account, address)
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "wpkh(")?;
        fmt::Display::fmt(&self.master, f)?;

        let mut derivation_path = self.derivation_path.0.clone();
        derivation_path.pop();

        fmt::Display::fmt(&DerivationPath(derivation_path), f)?;
        write!(f, "/*)")?;

        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct NewAddressRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: serde_json::Value,
}

impl NewAddressRequest {
    pub fn new(address_format: &str) -> Self {
        NewAddressRequest {
            jsonrpc: "1.0".to_string(),
            id: "getnewaddress".to_string(),
            method: "getnewaddress".to_string(),
            params: serde_json::json!(["", address_format]),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct NewAddressResponse {
    result: Option<Address>,
    error: Option<JsonRpcError>,
    id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct GenerateToAddressRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: serde_json::Value,
}

impl GenerateToAddressRequest {
    pub fn new(number: u32, address: Address) -> Self {
        GenerateToAddressRequest {
            jsonrpc: "1.0".to_string(),
            id: "generatetoaddress".to_string(),
            method: "generatetoaddress".to_string(),
            params: serde_json::json!([number, address]),
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct FundRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: serde_json::Value,
}

impl FundRequest {
    fn new(address: Address, amount: Amount) -> Self {
        FundRequest {
            jsonrpc: "1.0".to_string(),
            id: "sendtoaddress".to_string(),
            method: "sendtoaddress".to_string(),
            params: serde_json::json!([address, amount.as_btc().to_string()]),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct FundResponse {
    result: Option<sha256d::Hash>,
    error: Option<JsonRpcError>,
    id: String,
}

#[derive(Debug, serde::Deserialize, thiserror::Error)]
#[error("JSON-RPC request failed with code {code}: {message}")]
pub struct JsonRpcError {
    code: i64,
    message: String,
}

async fn fund(endpoint: &str, address: Address, amount: Amount) -> anyhow::Result<sha256d::Hash> {
    let client = reqwest::Client::new();

    let new_address = new_address(endpoint).await?;

    let _ = client
        .post(endpoint)
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&GenerateToAddressRequest::new(101, new_address))
        .send()
        .await
        .context("failed to generate blocks")?;

    let response: FundResponse = client
        .post(endpoint)
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&FundRequest::new(address, amount))
        .send()
        .await?
        .json::<FundResponse>()
        .await?;

    match response.error {
        None => match response.result {
            None => Err(anyhow::Error::msg(
                "no transaction hash returned without yielding error",
            )),
            Some(tx_hash) => Ok(tx_hash),
        },
        Some(error) => Err(anyhow::Error::new(error)),
    }
}

async fn new_address(endpoint: &str) -> anyhow::Result<Address> {
    let client = reqwest::Client::new();

    let response = client
        .post(endpoint)
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&NewAddressRequest::new("bech32"))
        .send()
        .await
        .context("failed to create new address")?
        .json::<NewAddressResponse>()
        .await?;

    match response.error {
        None => match response.result {
            None => Err(anyhow::Error::msg(
                "no address returned without yielding error",
            )),
            Some(address) => Ok(address),
        },
        Some(error) => Err(anyhow::Error::new(error)),
    }
}

fn derive_address(secret_key: secp256k1::SecretKey) -> Address {
    let public_key =
        secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &secret_key);
    derive_p2wpkh_regtest_address(public_key)
}

fn derive_p2wpkh_regtest_address(public_key: secp256k1::PublicKey) -> Address {
    Address::p2wpkh(
        &rust_bitcoin::PublicKey {
            compressed: true, // Only used for serialization
            key: public_key,
        },
        Network::Regtest,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn format_derivation_path() {
        let derivation_path = DerivationPath::bip44_bitcoin_testnet().unwrap();

        let to_string = derivation_path.to_string();
        assert_eq!(to_string, "/44'/1'/0'/0/0")
    }

    #[test]
    fn format_account() {
        let master = ExtendedPrivKey::from_str("tprv8ZgxMBicQKsPdypLixsdqgFVd55cqjtujNNPkHTHq963uLvbZj82cucKb4e3WPMxA2C4vCMZa7stjk2m4yzoMM7hB21bP7sHznToUEA7Qfb").unwrap();
        let account = Account::new(master).unwrap();

        let to_string = account.to_string();
        assert_eq!(
            to_string,
                "wpkh(tprv8ZgxMBicQKsPdypLixsdqgFVd55cqjtujNNPkHTHq963uLvbZj82cucKb4e3WPMxA2C4vCMZa7stjk2m4yzoMM7hB21bP7sHznToUEA7Qfb/44'/1'/0'/0/*)".to_owned(),
        )
    }

    #[test]
    fn generate_to_address_request_does_serialize() {
        let expected = r#"{"jsonrpc":"1.0","id":"generatetoaddress","method":"generatetoaddress","params":[101,"2MubReUTptB6isbuFmsRiN3BPHaeHpiAjQM"]}"#;

        let number = 101;
        let address = Address::from_str("2MubReUTptB6isbuFmsRiN3BPHaeHpiAjQM").unwrap();

        let request = GenerateToAddressRequest::new(number, address);
        let json = serde_json::to_string(&request).unwrap();

        assert_eq!(json, expected)
    }

    #[test]
    fn new_address_request_does_serialize() {
        let expected = r#"{"jsonrpc":"1.0","id":"getnewaddress","method":"getnewaddress","params":["","bech32"]}"#;
        let format = "bech32";

        let request = NewAddressRequest::new(format);
        let json = serde_json::to_string(&request).unwrap();

        assert_eq!(json, expected)
    }

    #[test]
    fn fund_request_does_serialize() {
        let expected = r#"{"jsonrpc":"1.0","id":"sendtoaddress","method":"sendtoaddress","params":["2MubReUTptB6isbuFmsRiN3BPHaeHpiAjQM","1"]}"#;
        let address = Address::from_str("2MubReUTptB6isbuFmsRiN3BPHaeHpiAjQM").unwrap();

        let request = FundRequest::new(address, Amount::ONE_BTC);
        let json = serde_json::to_string(&request).unwrap();

        assert_eq!(json, expected)
    }
}

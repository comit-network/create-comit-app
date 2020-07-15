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

use crate::docker::{
    self, docker_daemon_ip, free_local_port::free_local_port, DockerImage, LogMessage,
    DOCKER_NETWORK,
};
use serde::export::{Formatter, TryFrom};
use std::fmt;
use std::fmt::Display;

const IMAGE: &str = "coblox/bitcoin-core:0.20.0";

pub const USERNAME: &str = "bitcoin";
pub const PASSWORD: &str = "t68ej4UX2pB0cLlGwSwHFBLKxXYgomkXyFyxuBmm2U8=";

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

pub async fn new_bitcoind_instance() -> anyhow::Result<BitcoindInstance> {
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

    let http_port = free_local_port().await?;
    options_builder.expose(18443, "tcp", http_port as u32);

    let http_endpoint = BitcoindHttpEndpoint {
        port: http_port,
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

pub async fn mine_a_block(endpoint: BitcoindHttpEndpoint) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let new_address: NewAddressResponse = client
        .post(&endpoint.to_string())
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&NewAddressRequest::new("bech32"))
        .send()
        .await
        .context("failed to create new address")?
        .json::<NewAddressResponse>()
        .await?;
    assert!(new_address.error.is_none());

    let _ = client
        .post(&endpoint.to_string())
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&GenerateToAddressRequest::new(1, new_address.result))
        .send()
        .await?;

    Ok(())
}

pub struct DerivationPath {
    path: Vec<ChildNumber>,
}

impl Display for DerivationPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for i in 0..self.path.len() {
            let elem = self.path.get(i).unwrap();

            let separator = if i == self.path.len() - 1 { "" } else { "/" };

            match elem {
                ChildNumber::Normal { index } => {
                    write!(f, "{:?}{:0}", index, separator)?;
                }
                ChildNumber::Hardened { index } => {
                    write!(f, "{:?}h{:0}", index, separator)?;
                }
            }
        }

        Ok(())
    }
}

impl DerivationPath {
    pub fn bip44_bitcoin_testnet() -> anyhow::Result<Self> {
        Ok(Self {
            path: vec![
                ChildNumber::from_hardened_idx(44)?,
                ChildNumber::from_hardened_idx(1)?,
                ChildNumber::from_hardened_idx(0)?,
                ChildNumber::from_normal_idx(0)?,
                ChildNumber::from_normal_idx(0)?,
            ],
        })
    }
}

#[derive(Clone)]
pub struct Account {
    pub master: ExtendedPrivKey,
    first_account: rust_bitcoin::util::key::PrivateKey,
}

impl Account {
    fn new_random() -> anyhow::Result<Self> {
        let mut seed = [0u8; 32];
        thread_rng().fill_bytes(&mut seed);

        let master = ExtendedPrivKey::new_master(rust_bitcoin::Network::Regtest, &seed)
            .context("failed to generate new random extended private key from seed")?;

        // define derivation path to derive private keys from the master key
        let derivation_path =
            DerivationPath::bip44_bitcoin_testnet().context("failed to create derivation path")?;

        // derive a private key from the master key
        let priv_key = master
            .derive_priv(&Secp256k1::new(), &derivation_path.path)
            .map(|secret_key| secret_key.private_key)?;

        // it is not great to store derived data in here but since the derivation can fail, it is better to fail early instead of later
        Ok(Self {
            master,
            first_account: priv_key,
        })
    }

    fn first_account(&self) -> (rust_bitcoin::util::key::PrivateKey, rust_bitcoin::Address) {
        // derive an address from the private key
        let address = derive_address(self.first_account.key);

        (self.first_account, address)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct NewAddressRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<String>,
}

impl NewAddressRequest {
    pub fn new(address_format: &str) -> Self {
        let mut params = Vec::new();
        let label = "";
        params.push(label.to_owned());
        params.push(address_format.to_owned());

        NewAddressRequest {
            jsonrpc: "1.0".to_string(),
            id: "getnewaddress".to_string(),
            method: "getnewaddress".to_string(),
            params,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct GenerateToAddressRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<serde_json::Value>,
}

impl GenerateToAddressRequest {
    pub fn new(number: u32, address: Address) -> Self {
        let mut params = Vec::new();

        let number = serde_json::Value::Number(
            serde_json::Number::try_from(number).expect("can convert to number"),
        );
        assert!(number.is_u64());
        params.push(number);

        let address = serde_json::Value::String(address.to_string());
        params.push(address);

        GenerateToAddressRequest {
            jsonrpc: "1.0".to_string(),
            id: "generatetoaddress".to_string(),
            method: "generatetoaddress".to_string(),
            params,
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct FundRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<String>,
}

impl FundRequest {
    fn new(address: Address, amount: Amount) -> Self {
        FundRequest {
            jsonrpc: "1.0".to_string(),
            id: "fund".to_string(),
            method: "sendtoaddress".to_string(),
            params: vec![address.to_string(), amount.as_btc().to_string()],
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct FundResponse {
    result: Option<sha256d::Hash>,
    error: Option<JsonRpcError>,
    id: String,
}

#[derive(Debug, serde::Deserialize)]
struct NewAddressResponse {
    result: Address,
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

    let new_address = client
        .post(endpoint)
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&NewAddressRequest::new("bech32"))
        .send()
        .await
        .context("failed to create new address")?
        .json::<NewAddressResponse>()
        .await?;
    assert!(new_address.error.is_none());

    let _ = client
        .post(endpoint)
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&GenerateToAddressRequest::new(101, new_address.result))
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
        assert_eq!(to_string, "44h/1h/0h/0/0")
    }

    #[test]
    fn generate_to_address_request_does_not_panic() {
        let number = 101;
        let address = Address::from_str("2MubReUTptB6isbuFmsRiN3BPHaeHpiAjQM").unwrap();

        let _ = GenerateToAddressRequest::new(number, address);
    }

    #[test]
    fn new_address_request_does_not_panic() {
        let format = "bech32";

        let _ = NewAddressRequest::new(format);
    }
}

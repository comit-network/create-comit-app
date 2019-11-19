use crate::docker::{ExposedPorts, Image};
use reqwest::r#async::Client;
use rust_bitcoin::{self, hashes::sha256d, Address, Amount, Network};
use tokio::prelude::future::Future;

pub const P2P_URI_KEY: &str = "BITCOIN_P2P_URI";
pub const HTTP_URL_KEY: &str = "BITCOIN_NODE_RPC_URL";

pub struct BitcoinNode {
    pub username: String,
    pub password: String,
    pub endpoint: String,
}

impl BitcoinNode {
    const USERNAME: &'static str = "bitcoin";
    const PASSWORD: &'static str = "t68ej4UX2pB0cLlGwSwHFBLKxXYgomkXyFyxuBmm2U8=";
}

impl Image for BitcoinNode {
    const IMAGE: &'static str = "coblox/bitcoin-core:0.17.0";
    const LOG_READY: &'static str = "Flushed wallet.dat";

    fn arguments_for_create() -> Vec<&'static str> {
        vec![
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
    ]
    }

    fn expose_ports(_: &str) -> Vec<ExposedPorts> {
        vec![
            ExposedPorts {
                for_client: true,
                srcport: 18443,
                env_file_key: HTTP_URL_KEY.to_string(),
                env_file_value: Box::new(|port| format!("http://localhost:{}", port)),
            },
            ExposedPorts {
                for_client: false,
                srcport: 18444,
                env_file_key: P2P_URI_KEY.to_string(),
                env_file_value: Box::new(|port| format!("127.0.0.1:{}", port)),
            },
        ]
    }

    fn new(endpoint: Option<String>) -> Self {
        let endpoint: String = endpoint.unwrap_or_else(|| {
            panic!("Internal Error: Url for bitcoin client should have been set.");
        });

        Self {
            username: Self::USERNAME.to_string(),
            password: Self::PASSWORD.to_string(),
            endpoint,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct GenerateQuery {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<u32>,
}

impl GenerateQuery {
    pub fn new(number: u32) -> Self {
        GenerateQuery {
            jsonrpc: "1.0".to_string(),
            id: "generate".to_string(),
            method: "generate".to_string(),
            params: vec![number],
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct FundQuery {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<String>,
}

impl FundQuery {
    fn new(address: Address, amount: Amount) -> Self {
        FundQuery {
            jsonrpc: "1.0".to_string(),
            id: "fund".to_string(),
            method: "sendtoaddress".to_string(),
            params: vec![address.to_string(), amount.as_btc().to_string()],
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct FundResponse {
    result: sha256d::Hash,
    error: Option<String>,
    id: String,
}

pub fn fund(
    username: String,
    password: String,
    endpoint: String,
    address: Address,
    amount: Amount,
) -> impl Future<Item = sha256d::Hash, Error = anyhow::Error> {
    let generate_req = GenerateQuery::new(101);
    let fund_req = FundQuery::new(address, amount);

    Box::new(
        Client::new()
            .post(&endpoint)
            .basic_auth(&username, Some(password.clone()))
            .json(&generate_req)
            .send()
            .and_then({
                let endpoint = endpoint.clone();
                let username = username.clone();
                let password = Some(password.clone());

                move |_| {
                    Client::new()
                        .post(&endpoint)
                        .basic_auth(username, password)
                        .json(&fund_req)
                        .send()
                        .and_then(|mut response| response.json::<FundResponse>())
                        .and_then(|response| Ok(response.result))
                }
            })
            .from_err(),
    )
}

pub fn derive_address(secret_key: secp256k1::SecretKey) -> Address {
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

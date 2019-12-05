use crate::docker::{
    self, bitcoin_network_message_codec::RawNetworkMessageCodec, docker_daemon_ip,
    free_local_port::free_local_port, DockerImage, LogMessage, DOCKER_NETWORK,
};
use anyhow::Context;
use futures::{
    compat::{Future01CompatExt, Sink01CompatExt, Stream01CompatExt},
    SinkExt, StreamExt,
};
use reqwest::r#async::Client;
use rust_bitcoin::{
    self,
    hashes::sha256d,
    network::{
        message::{NetworkMessage, RawNetworkMessage},
        message_blockdata::InvType,
        message_network::VersionMessage,
    },
    util::bip32::{ChildNumber, ExtendedPrivKey},
    Address, Amount, Network,
};
use secp256k1::{
    rand::{thread_rng, Rng},
    Secp256k1,
};
use shiplift::ContainerOptions;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::SystemTime,
};
use tokio::{codec::Decoder, prelude::Stream};

const IMAGE: &str = "coblox/bitcoin-core:0.17.0";

const USERNAME: &str = "bitcoin";
const PASSWORD: &str = "t68ej4UX2pB0cLlGwSwHFBLKxXYgomkXyFyxuBmm2U8=";

#[derive(derive_more::Display, Copy, Clone)]
#[display(fmt = "{}:{}", ip, port)]
pub struct BitcoindP2PUri {
    port: u16,
    ip: Ipv4Addr,
}

impl From<BitcoindP2PUri> for SocketAddr {
    fn from(bitcoind_p2puri: BitcoindP2PUri) -> Self {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, bitcoind_p2puri.port))
    }
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
    .await?;

    let account_0 = fund_new_account(http_endpoint).await?;
    let account_1 = fund_new_account(http_endpoint).await?;

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

pub async fn new_miner(instance: BitcoindInstance) -> anyhow::Result<()> {
    let socket_addr = SocketAddr::from(instance.p2p_uri);
    let stream = tokio::net::TcpStream::connect(&socket_addr)
        .compat()
        .await?;

    let (sink, stream) = RawNetworkMessageCodec.framed(stream).split();
    let mut stream = stream.compat();
    let mut sink = sink.sink_compat();

    let _ = sink.send(announce_version_message(&socket_addr)).await;

    loop {
        if let Some(Ok(message)) = stream.next().await {
            match message.payload {
                // Ack the version of the node to kick of the communication
                NetworkMessage::Version(_) => {
                    let _ = sink
                        .send(RawNetworkMessage {
                            magic: Network::Regtest.magic(),
                            payload: NetworkMessage::Verack,
                        })
                        .await;
                }
                // If we get told about a new transaction, tell the node to mine a block
                NetworkMessage::Inv(inventory)
                    if inventory.iter().any(|i| i.inv_type == InvType::Transaction) =>
                {
                    let _ = mine_a_block(instance.http_endpoint).await;
                }
                // Ignore all other incoming messages
                _ => {}
            }
        }
    }
}

fn announce_version_message(socket_addr: &SocketAddr) -> RawNetworkMessage {
    #[allow(clippy::clippy::cast_possible_wrap)]
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let services = rust_bitcoin::network::constants::SERVICES;

    RawNetworkMessage {
        magic: Network::Regtest.magic(),
        payload: NetworkMessage::Version(VersionMessage {
            version: rust_bitcoin::network::constants::PROTOCOL_VERSION,
            services,
            timestamp: now,
            receiver: rust_bitcoin::network::address::Address::new(&socket_addr, 0),
            sender: rust_bitcoin::network::address::Address::new(
                &SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
                services,
            ),
            nonce: 0,
            user_agent: "create-comit-app".to_string(),
            start_height: 0,
            relay: false,
        }),
    }
}

async fn mine_a_block(endpoint: BitcoindHttpEndpoint) -> anyhow::Result<()> {
    let _ = reqwest::r#async::Client::new()
        .post(&endpoint.to_string())
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&GenerateRequest::new(1))
        .send()
        .compat()
        .await?;

    Ok(())
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
        let derivation_path = vec![
            ChildNumber::from_hardened_idx(44)?,
            ChildNumber::from_hardened_idx(1)?,
            ChildNumber::from_hardened_idx(0)?,
            ChildNumber::from_normal_idx(0)?,
            ChildNumber::from_normal_idx(0)?,
        ];

        // derive a private key from the master key
        let priv_key = master
            .derive_priv(&Secp256k1::new(), &derivation_path)
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
pub struct GenerateRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<u32>,
}

impl GenerateRequest {
    pub fn new(number: u32) -> Self {
        GenerateRequest {
            jsonrpc: "1.0".to_string(),
            id: "generate".to_string(),
            method: "generate".to_string(),
            params: vec![number],
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
    result: sha256d::Hash,
    error: Option<String>,
    id: String,
}

async fn fund(endpoint: &str, address: Address, amount: Amount) -> anyhow::Result<sha256d::Hash> {
    let client = Client::new();

    let _ = client
        .post(endpoint)
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&GenerateRequest::new(101))
        .send()
        .compat()
        .await?;

    let mut response = client
        .post(endpoint)
        .basic_auth(USERNAME, Some(PASSWORD))
        .json(&FundRequest::new(address, amount))
        .send()
        .compat()
        .await?;

    let response = response.json::<FundResponse>().compat().await?;

    Ok(response.result)
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

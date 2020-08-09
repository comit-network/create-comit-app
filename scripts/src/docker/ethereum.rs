use std::{net::Ipv4Addr, time::Duration};

use anyhow::Context;
use futures::compat::Future01CompatExt;
use num256::Uint256;
use secp256k1::{rand::thread_rng, SecretKey};
use shiplift::ContainerOptions;
use web3::{
    api::Web3,
    transports::Http,
    types::{Bytes, TransactionReceipt, U256},
};

use lazy_static::lazy_static;

use crate::{
    config,
    docker::{
        self, docker_daemon_ip, free_local_port::free_local_port, DockerImage, LogMessage,
        DOCKER_NETWORK,
    },
};

pub const TOKEN_CONTRACT: &str = include_str!("../../erc20_token/build/contract.hex");
pub const CONTRACT_ABI: &str = include_str!("../../erc20_token/build/abi.json");

const IMAGE: &str = "coblox/parity-poa:v2.5.9-stable";

lazy_static! {
    static ref DEV_ACCOUNT: web3::types::Address = "00a329c0648769a73afac7f9381e08fb43dbea72"
        .parse()
        .expect("Should not fail: Could not parse DEV account address");
    static ref DEV_ACCOUNT_PRIVATE_KEY: clarity::PrivateKey = clarity::PrivateKey::from(
        hex_literal::hex!("4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7")
    );
}

#[derive(derive_more::Display, Copy, Clone)]
#[display(fmt = "http://{}:{}", ip, port)]
pub struct ParityHttpEndpoint {
    ip: Ipv4Addr,
    port: u16,
}

pub struct ParityInstance {
    pub http_endpoint: ParityHttpEndpoint,
    pub account_0: Account,
    pub account_1: Account,
    pub erc20_contract_address: clarity::Address,
}

pub async fn new_parity_instance(
    config: Option<config::Ethereum>,
) -> anyhow::Result<ParityInstance> {
    let mut options_builder = ContainerOptions::builder(IMAGE);
    options_builder.name("ethereum");
    options_builder.network_mode(DOCKER_NETWORK);

    let http_port = free_local_port()
        .await
        .context("failed to acquire free local port")?;
    options_builder.expose(8545, "tcp", http_port as u32);

    let http_endpoint = ParityHttpEndpoint {
        port: http_port,
        ip: docker_daemon_ip()?,
    };

    let options = options_builder.build();

    docker::start(
        DockerImage(IMAGE),
        options,
        LogMessage("Public node URL:"),
        vec![],
    )
    .await
    .context("failed to start container")?;

    let account_0 = fund_new_account(http_endpoint)
        .await
        .context("failed to fund first account")?;
    let account_1 = fund_new_account(http_endpoint)
        .await
        .context("failed to fund second account")?;

    if let Some(config) = config.clone() {
        for address in config.addresses_to_fund {
            fund_address(http_endpoint, address)
                .await
                .context("failed to fund config account")?;
        }
    }

    let mut addresses = config
        .map(|config| config.addresses_to_fund)
        .unwrap_or_default();

    addresses.push(derive_address(account_0)?);
    addresses.push(derive_address(account_1)?);
    let contract_address = new_erc20_contract(http_endpoint, addresses).await?;

    Ok(ParityInstance {
        http_endpoint,
        account_0,
        account_1,
        erc20_contract_address: contract_address,
    })
}

#[derive(Clone, Copy)]
pub struct Account {
    pub private_key: SecretKey,
}

impl Account {
    fn new_random() -> Self {
        Self {
            private_key: SecretKey::new(&mut thread_rng()),
        }
    }
}

async fn fund_new_account(endpoint: ParityHttpEndpoint) -> anyhow::Result<Account> {
    let (_event_loop_handle, transport) = Http::new(&endpoint.to_string())
        .context("unable to initialize http transport to ethereum node")?;
    let client = Web3::new(transport);

    let account = Account::new_random();
    let address = derive_address(account)?;

    send_transaction(
        client.clone(),
        Some(address),
        30_000,
        U256::from(1000u128 * 10u128.pow(18)),
        Vec::new(),
    )
    .await
    .with_context(|| {
        format!(
            "failed to send transaction for funding account {:x} with ether to {}",
            address,
            endpoint.to_string()
        )
    })?;

    Ok(account)
}

async fn fund_address(
    endpoint: ParityHttpEndpoint,
    address: clarity::Address,
) -> anyhow::Result<()> {
    let (_event_loop_handle, transport) = Http::new(&endpoint.to_string())
        .context("unable to initialize http transport to ethereum node")?;
    let client = Web3::new(transport);

    send_transaction(
        client.clone(),
        Some(address),
        30_000,
        U256::from(1000u128 * 10u128.pow(18)),
        Vec::new(),
    )
    .await
    .with_context(|| {
        format!(
            "failed to send transaction for funding account {:x} with ether to {}",
            address,
            endpoint.to_string()
        )
    })?;

    Ok(())
}

async fn new_erc20_contract(
    endpoint: ParityHttpEndpoint,
    addresses: Vec<clarity::Address>,
) -> anyhow::Result<clarity::Address> {
    let (_event_loop_handle, transport) = Http::new(&endpoint.to_string())?;
    let client = Web3::new(transport);

    let contract_address = deploy_erc20_contract(client.clone()).await?;

    for address in addresses {
        let transfer = transfer_fn(
            address,
            Uint256::from(100u128) * Uint256::from(10u128.pow(18)),
        );
        send_transaction(
            client.clone(),
            Some(contract_address),
            100_000,
            U256::from(0u64),
            transfer,
        )
        .await
        .with_context(|| {
            format!(
                "failed to send transaction for funding account {:x} with erc20 to {}",
                address,
                endpoint.to_string()
            )
        })?;
    }

    Ok(contract_address)
}

async fn deploy_erc20_contract(client: Web3<Http>) -> anyhow::Result<clarity::Address> {
    let data = TOKEN_CONTRACT[2..].trim(); // remove the 0x in the front and any whitespace
    let erc20_contract = hex::decode(data).context("token contract should be valid hex")?;

    let receipt = send_transaction(client, None, 10_000_000, U256::from(0), erc20_contract).await?;

    let contract_address = receipt
        .contract_address
        .context("contract_address not present, invalid deployment transaction?")?;

    Ok(clarity::Address::from(contract_address.0))
}

fn transfer_fn(address: clarity::Address, amount: Uint256) -> Vec<u8> {
    clarity::abi::encode_call(
        "transfer(address,uint256)",
        &[
            clarity::abi::Token::Address(address),
            clarity::abi::Token::Uint(amount),
        ],
    )
}

async fn send_transaction(
    client: Web3<Http>,
    to: Option<clarity::Address>,
    gas_limit: u64,
    amount: U256,
    data: Vec<u8>,
) -> anyhow::Result<TransactionReceipt> {
    let chain_id = get_chain_id(client.clone()).await?;
    let tx = clarity::Transaction {
        nonce: client
            .eth()
            .transaction_count(*DEV_ACCOUNT, None)
            .compat()
            .await?
            .as_u64()
            .into(),
        gas_price: 0u32.into(),
        gas_limit: gas_limit.into(),
        to: to.unwrap_or_default(),
        value: <[u8; 32]>::from(amount).into(),
        data,
        signature: None,
    };
    let signed_raw_tx = tx.sign(&*DEV_ACCOUNT_PRIVATE_KEY, Some(chain_id.into()));
    let serialized_tx = signed_raw_tx
        .to_bytes()
        .map_err(|e| anyhow::anyhow!("failed to serialize transaction {:?}", e))?;

    let tx_id = client
        .send_raw_transaction_with_confirmation(Bytes(serialized_tx), Duration::from_millis(100), 1)
        .compat()
        .await?;

    Ok(tx_id)
}

async fn get_chain_id(client: Web3<Http>) -> anyhow::Result<u16> {
    let network = client.net().version().compat().await?;

    network
        .parse::<u16>()
        .with_context(|| format!("{} is not a valid chain-id", network))
}

fn derive_address(account: Account) -> anyhow::Result<clarity::Address> {
    let address = clarity::PrivateKey::from_slice(&account.private_key[..])
        .map_err(|e| anyhow::anyhow!("failed to create private key from slice {:?}", e))?
        .to_public_key()
        .map_err(|e| anyhow::anyhow!("failed to turn private key into an address {:?}", e))?;

    Ok(address)
}

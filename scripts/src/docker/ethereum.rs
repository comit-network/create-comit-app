use std::net::Ipv4Addr;

use crate::{
    config,
    docker::{self, docker_daemon_ip, DockerImage, LogMessage, DOCKER_NETWORK},
};
use anyhow::Context;
use num256::Uint256;
use secp256k1::{rand::thread_rng, SecretKey};
use shiplift::ContainerOptions;
use std::time::Duration;
use web3::{
    api::Web3,
    confirm::send_transaction_with_confirmation,
    transports::Http,
    types::{Address, TransactionReceipt, TransactionRequest, H160, U256},
};

pub const TOKEN_CONTRACT: &str = include_str!("../../erc20_token/build/contract.hex");
pub const CONTRACT_ABI: &str = include_str!("../../erc20_token/build/abi.json");

const IMAGE: &str = "ethereum/client-go:v1.9.18";

const CHAIN_ID: &str = "1337";
const HTTP_PORT: u16 = 8545;

#[derive(derive_more::Display, Copy, Clone)]
#[display(fmt = "http://{}:{}", ip, port)]
pub struct GethHttpEndpoint {
    ip: Ipv4Addr,
    port: u16,
}

pub struct GethInstance {
    pub http_endpoint: GethHttpEndpoint,
    pub account_0: Account,
    pub account_1: Account,
    pub erc20_contract_address: Address,
}

pub async fn new_geth_instance(config: Option<config::Ethereum>) -> anyhow::Result<GethInstance> {
    let mut options_builder = ContainerOptions::builder(IMAGE);
    options_builder.name("ethereum");
    options_builder.network_mode(DOCKER_NETWORK);
    options_builder.cmd(vec![
        "--dev",
        "--dev.period=1", // generates a block every X seconds
        format!("--networkid={}", CHAIN_ID).as_str(),
        "--http",
        "--http.addr=0.0.0.0",
        format!("--http.port={}", HTTP_PORT).as_str(),
        "--http.api=eth,net,web3,personal",
        "--verbosity=4",
        "--allow-insecure-unlock",
    ]);

    options_builder.expose(HTTP_PORT as u32, "tcp", HTTP_PORT as u32);

    let http_endpoint = GethHttpEndpoint {
        port: HTTP_PORT,
        ip: docker_daemon_ip()?,
    };

    let options = options_builder.build();

    docker::start(
        DockerImage(IMAGE),
        options,
        LogMessage("mined potential block"),
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

    Ok(GethInstance {
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

async fn fund_new_account(endpoint: GethHttpEndpoint) -> anyhow::Result<Account> {
    let transport = Http::new(&endpoint.to_string())
        .context("unable to initialize http transport to ethereum node")?;
    let client = Web3::new(transport);

    let account = Account::new_random();
    let address = derive_address(account)?;

    send_transaction(
        client,
        Some(address),
        30_000,
        U256::from(1000u128 * 10u128.pow(18)),
        Vec::new(),
    )
    .await
    .with_context(|| {
        format!(
            "failed to fund new account {:x} with ether to {}",
            address,
            endpoint.to_string(),
        )
    })?;

    Ok(account)
}

async fn fund_address(endpoint: GethHttpEndpoint, address: Address) -> anyhow::Result<()> {
    let transport = Http::new(&endpoint.to_string())
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
    endpoint: GethHttpEndpoint,
    addresses: Vec<Address>,
) -> anyhow::Result<Address> {
    let transport = Http::new(&endpoint.to_string())?;
    let client = Web3::new(transport);

    let contract_address = deploy_erc20_contract(client.clone()).await?;

    for address in addresses {
        let transfer = transfer_fn(
            clarity::Address::from(address.0),
            Uint256::from(100000u128) * Uint256::from(10u128.pow(18)),
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

async fn deploy_erc20_contract(client: Web3<Http>) -> anyhow::Result<Address> {
    let data = TOKEN_CONTRACT[2..].trim(); // remove the 0x in the front and any whitespace
    let erc20_contract = hex::decode(data).context("token contract should be valid hex")?;

    let receipt = send_transaction(client, None, 4_000_000, U256::from(0), erc20_contract).await?;

    let contract_address = receipt
        .contract_address
        .context("contract_address not present, invalid deployment transaction?")?;

    Ok(contract_address)
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
    to: Option<Address>,
    gas_limit: u64,
    amount: U256,
    data: Vec<u8>,
) -> anyhow::Result<TransactionReceipt> {
    let dev_account = client.eth().coinbase().await?;
    let unlock = client
        .personal()
        .unlock_account(dev_account, "", None)
        .await?;
    if !unlock {
        anyhow::bail!("Failed to unlock dev-account")
    }

    let nonce = client
        .eth()
        .transaction_count(dev_account, None)
        .await?
        .as_u64()
        .into();

    let request = TransactionRequest {
        from: dev_account,
        to,
        gas: Some(gas_limit.into()),
        gas_price: None,
        value: Some(<[u8; 32]>::from(amount).into()),
        data: Some(web3::types::Bytes::from(data)),
        nonce: Some(nonce),
        condition: None,
    };

    // Use send_transaction_with_confirmation over send_transaction to ensure that nonce is increased correctly and receipt can be retrieved
    let receipt = send_transaction_with_confirmation(
        client.transport(),
        request,
        Duration::from_millis(500),
        1,
    )
    .await?;

    Ok(receipt)
}

fn derive_address(account: Account) -> anyhow::Result<Address> {
    let address = clarity::PrivateKey::from_slice(&account.private_key[..])
        .map_err(|e| anyhow::anyhow!("failed to create private key from slice {:?}", e))?
        .to_public_key()
        .map_err(|e| anyhow::anyhow!("failed to turn private key into an address {:?}", e))?;

    let mut data: [u8; 20] = Default::default();
    data.copy_from_slice(&address.as_bytes());
    Ok(H160(data))
}

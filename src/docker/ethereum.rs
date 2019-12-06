use crate::docker::{
    self, free_local_port::free_local_port, DockerImage, LogMessage, DOCKER_NETWORK,
};
use emerald_rs::PrivateKey;
use futures::compat::Future01CompatExt;
use lazy_static::lazy_static;
use secp256k1::{rand::thread_rng, SecretKey};
use shiplift::ContainerOptions;
use std::{io::Cursor, time::Duration};
use web3::{
    api::Web3,
    transports::Http,
    types::{Address, Bytes, TransactionReceipt, H160, U256},
};

pub const TOKEN_CONTRACT: &str = include_str!("../../erc20_token/build/contract.hex");
pub const CONTRACT_ABI: &str = include_str!("../../erc20_token/build/abi.json");

const IMAGE: &str = "coblox/parity-poa:v2.5.9-stable";

lazy_static! {
    static ref DEV_ACCOUNT: web3::types::Address = "00a329c0648769a73afac7f9381e08fb43dbea72"
        .parse()
        .expect("Should not fail: Could not parse DEV account address");
    static ref DEV_ACCOUNT_PRIVATE_KEY: emerald_rs::PrivateKey = emerald_rs::PrivateKey::try_from(
        &hex_literal::hex!("4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7")
    )
    .expect("Should not fail: parsing static private key ");
}

#[derive(derive_more::Display, Copy, Clone)]
#[display(fmt = "http://localhost:{}", port)]
pub struct ParityHttpEndpoint {
    port: u16,
}

pub struct ParityInstance {
    pub http_endpoint: ParityHttpEndpoint,
    pub account_0: Account,
    pub account_1: Account,
    pub erc20_contract_address: web3::types::Address,
}

pub async fn new_parity_instance() -> anyhow::Result<ParityInstance> {
    let mut options_builder = ContainerOptions::builder(IMAGE);
    options_builder.name("ethereum");
    options_builder.network_mode(DOCKER_NETWORK);

    let http_port = free_local_port().await?;
    options_builder.expose(8545, "tcp", http_port as u32);

    let http_endpoint = ParityHttpEndpoint { port: http_port };

    let options = options_builder.build();

    docker::start(DockerImage(IMAGE), options, LogMessage("Public node URL:")).await?;

    let account_0 = fund_new_account(http_endpoint).await?;
    let account_1 = fund_new_account(http_endpoint).await?;
    let contract_address = new_erc20_contract(http_endpoint, vec![account_0, account_1]).await?;

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
    let (_event_loop_handle, transport) = Http::new(&endpoint.to_string())?;
    let client = Web3::new(transport);

    let account = Account::new_random();

    send_transaction(
        client.clone(),
        Some(derive_address(account.private_key)),
        30_000,
        U256::from(1000u128 * 10u128.pow(18)),
        Vec::new(),
    )
    .await?;

    Ok(account)
}

async fn new_erc20_contract(
    endpoint: ParityHttpEndpoint,
    accounts: Vec<Account>,
) -> anyhow::Result<Address> {
    let (_event_loop_handle, transport) = Http::new(&endpoint.to_string())?;
    let client = Web3::new(transport);

    let contract_address = deploy_erc20_contract(client.clone()).await?;

    for address in accounts
        .into_iter()
        .map(|account| derive_address(account.private_key))
    {
        let transfer = transfer_fn(address, U256::from(100u128 * 10u128.pow(18))).map_err(|e| {
            eprintln!("Failed to generate ERC20 transfer fn data: {:?}", e);
            web3::Error::Internal
        })?;
        send_transaction(
            client.clone(),
            Some(contract_address),
            100_000,
            U256::from(0u64),
            transfer,
        )
        .await?;
    }

    Ok(contract_address)
}

async fn deploy_erc20_contract(client: Web3<Http>) -> Result<Address, web3::Error> {
    let data = TOKEN_CONTRACT[2..].trim(); // remove the 0x in the front and any whitespace
    let erc20_contract = hex::decode(data).expect("token contract should be valid hex");

    let receipt = send_transaction(client, None, 10_000_000, U256::from(0), erc20_contract).await?;

    let contract_address = receipt
        .contract_address
        .expect("we deployed a contract, should have contract address");

    Ok(contract_address)
}

fn transfer_fn(address: Address, amount: U256) -> Result<Vec<u8>, ethabi::Error> {
    ethabi::Contract::load(Cursor::new(CONTRACT_ABI))?
        .function("transfer")?
        .encode_input(&[ethabi::Token::Address(address), ethabi::Token::Uint(amount)])
}

async fn send_transaction(
    client: Web3<Http>,
    to: Option<Address>,
    gas_limit: u64,
    amount: U256,
    data: Vec<u8>,
) -> Result<TransactionReceipt, web3::Error> {
    let chain_id = get_chain_id(client.clone()).await?;
    let tx = emerald_rs::Transaction {
        nonce: client
            .eth()
            .transaction_count(*DEV_ACCOUNT, None)
            .compat()
            .await?
            .as_u64(),
        gas_price: [0u8; 32],
        gas_limit,
        to: to.map(|a| emerald_rs::Address(a.0)),
        value: amount.into(),
        data,
    };
    let signed_raw_tx = tx
        .to_signed_raw(*DEV_ACCOUNT_PRIVATE_KEY, chain_id)
        .expect("signing transaction should work");

    client
        .send_raw_transaction_with_confirmation(Bytes(signed_raw_tx), Duration::from_millis(100), 1)
        .compat()
        .await
}

async fn get_chain_id(client: Web3<Http>) -> Result<u8, web3::Error> {
    let network = client.net().version().compat().await?;
    network.parse::<u8>().map_err(|e| {
        web3::Error::InvalidResponse(format!(
            "{} is not a valid chain id because it cannot be parsed as a u8: {:?}",
            network, e
        ))
    })
}

fn derive_address(secret_key: secp256k1::SecretKey) -> Address {
    let address = PrivateKey::try_from(&secret_key[..])
        .expect("can never happen")
        .to_address();

    H160(address.0)
}

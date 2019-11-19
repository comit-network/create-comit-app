use crate::docker::{ExposedPorts, Image};
use emerald_rs::PrivateKey;
use ethabi::Token;
use futures::compat::Future01CompatExt;
use lazy_static::lazy_static;
use secp256k1::SecretKey;
use std::{io::Cursor, time::Duration};
use web3::{
    api::Web3,
    transports::{EventLoopHandle, Http},
    types::{Address, Bytes, TransactionReceipt, H160, U256},
};

pub const TOKEN_CONTRACT: &str = include_str!("../../../erc20_token/build/contract.hex");
pub const CONTRACT_ABI: &str = include_str!("../../../erc20_token/build/abi.json");
pub const HTTP_URL_KEY: &str = "ETHEREUM_NODE_HTTP_URL";

lazy_static! {
    static ref DEV_ACCOUNT: web3::types::Address = "00a329c0648769a73afac7f9381e08fb43dbea72"
        .parse()
        .expect("Should not fail: Could not parse DEV account address");
    static ref DEV_ACCOUNT_PRIVATE_KEY: emerald_rs::PrivateKey = emerald_rs::PrivateKey::try_from(
        &hex_literal::hex!("4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7")
    )
    .expect("Should not fail: parsing static private key ");
}

pub struct EthereumNode {
    pub http_client: Web3<Http>,
    _event_loop: EventLoopHandle,
}

impl Image for EthereumNode {
    const IMAGE: &'static str = "coblox/parity-poa:v2.5.9-stable";
    const LOG_READY: &'static str = "Public node URL:";

    fn arguments_for_create() -> Vec<&'static str> {
        vec![]
    }

    // TODO: Probably actually use the name instead of HTTP_URL_KEY
    fn expose_ports(_: &str) -> Vec<ExposedPorts> {
        vec![ExposedPorts {
            for_client: true,
            srcport: 8545,
            env_file_key: HTTP_URL_KEY.to_string(),
            env_file_value: Box::new(|port| format!("http://localhost:{}", port)),
        }]
    }

    fn new(endpoint: Option<String>) -> Self {
        let endpoint: String = endpoint.unwrap_or_else(|| {
            panic!("Internal Error: Url for web3 client should have been set.");
        });
        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let http_client = Web3::new(transport);
        Self {
            http_client,
            _event_loop,
        }
    }
}

pub async fn fund_ether(
    client: Web3<Http>,
    keys: Vec<SecretKey>,
    amount: U256,
) -> Result<(), web3::Error> {
    for address in keys.into_iter().map(derive_address) {
        send_transaction(client.clone(), Some(address), 30_000, amount, Vec::new()).await?;
    }

    Ok(())
}

pub async fn fund_erc20(
    client: Web3<Http>,
    keys: Vec<SecretKey>,
    amount: U256,
) -> Result<Address, web3::Error> {
    let contract_address = deploy_erc20_contract(client.clone()).await?;

    for address in keys.into_iter().map(derive_address) {
        let transfer = transfer_fn(address, amount).map_err(|e| {
            eprintln!("Failed to generate ERC20 transfer fn data: {:?}", e);
            web3::Error::Internal
        })?;
        send_transaction(
            client.clone(),
            Some(contract_address),
            100_000,
            U256::from(0),
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
        .encode_input(&[Token::Address(address), Token::Uint(amount)])
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

pub fn derive_address(secret_key: secp256k1::SecretKey) -> Address {
    let address = PrivateKey::try_from(&secret_key[..])
        .expect("can never happen")
        .to_address();

    H160(address.0)
}

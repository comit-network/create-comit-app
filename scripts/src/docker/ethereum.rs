use std::net::Ipv4Addr;

use crate::{
    config,
    docker::{self, docker_daemon_ip, DockerImage, LogMessage, DOCKER_NETWORK},
};
use anyhow::Context;
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

struct GethClient {
    client: Web3<Http>,
    endpoint_url: String,
}

impl GethClient {
    fn new(endpoint: &GethHttpEndpoint) -> anyhow::Result<Self> {
        let url = endpoint.to_string();
        let transport =
            Http::new(&url).context("unable to initialize http transport to ethereum node")?;
        Ok(GethClient {
            client: Web3::new(transport),
            endpoint_url: url,
        })
    }
    async fn fund_ethereum(&self, address: Address) -> anyhow::Result<()> {
        send_transaction(
            self.client.clone(),
            Some(address),
            30_000,
            U256::from(1000u128 * 10u128.pow(18)),
            Vec::new(),
        )
        .await
        .with_context(|| {
            format!(
                "failed to send transaction for funding account {:x} with ether to {}",
                address, self.endpoint_url
            )
        })?;

        Ok(())
    }

    async fn fund_erc20(&self, address: Address, contract_address: Address) -> anyhow::Result<()> {
        send_transaction(
            self.client.clone(),
            Some(contract_address),
            100_000,
            U256::from(0u64),
            Vec::new(),
        )
        .await
        .with_context(|| {
            format!(
                "failed to send transaction for funding account {:x} with erc20 to {}",
                address, self.endpoint_url
            )
        })?;

        Ok(())
    }

    async fn deploy_erc20_contract(&self) -> anyhow::Result<Address> {
        let data = TOKEN_CONTRACT[2..].trim(); // remove the 0x in the front and any whitespace
        let erc20_contract = hex::decode(data).context("token contract should be valid hex")?;

        let receipt = send_transaction(
            self.client.clone(),
            None,
            10_000_000,
            U256::from(0),
            erc20_contract,
        )
        .await?;

        let contract_address = receipt
            .contract_address
            .context("contract_address not present, invalid deployment transaction?")?;

        Ok(contract_address)
    }
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

    let client = GethClient::new(&http_endpoint)?;

    let contract_address = client
        .deploy_erc20_contract()
        .await
        .context("failed to deploy erc20 contract")?;

    let account_0 = Account::new_random();
    let account_1 = Account::new_random();

    let mut addresses = config
        .clone()
        .map(|config| config.addresses_to_fund)
        .unwrap_or_default();

    addresses.push(derive_address(account_0)?);
    addresses.push(derive_address(account_1)?);

    for address in addresses {
        client
            .fund_ethereum(address)
            .await
            .context("failed to fund account with ethereum")?;
        client
            .fund_erc20(address, contract_address)
            .await
            .context("failed to fund account with erc20")?;
    }

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

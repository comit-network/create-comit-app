use envfile::EnvFile;
use futures::stream::Stream;
use shiplift::{ContainerOptions, Docker, LogsOptions, RmContainerOptions};
use std::path::PathBuf;
use tiny_keccak;
use web3::transports::EventLoopHandle;
use web3::{
    api::Web3,
    futures::Future,
    transports::Http,
    types::{Address, TransactionRequest, H256, U256},
};

pub const HTTP_URL_KEY: &str = "ETHEREUM_NODE_HTTP_URL";

pub struct EthereumNode {
    pub container_id: String,
    pub http_client: Web3<Http>,
    _event_loop: EventLoopHandle,
}

// TODO: Move all envfile stuff outside
// TODO: Move free_local_port outside
impl EthereumNode {
    pub fn start(
        envfile_path: PathBuf,
    ) -> impl Future<Item = Self, Error = shiplift::errors::Error> {
        let http_port: u32 = port_check::free_local_port().unwrap().into();
        let http_url = format!("http://localhost:{}", http_port);

        let docker = Docker::new();
        let image = "parity/parity:v2.5.0";
        docker
            .containers()
            .create(
                &ContainerOptions::builder(image)
                    .cmd(vec![
                        "--config=dev",
                        "--jsonrpc-apis=all",
                        "--unsafe-expose",
                        "--tracing=on",
                        "--jsonrpc-cors=all",
                    ])
                    .expose(8545, "tcp", http_port)
                    .build(),
            )
            .and_then({
                let docker = docker.clone();
                move |container| {
                    let id = container.id;
                    docker.containers().get(&id).start().map(|_| id)
                }
            })
            .and_then({
                let docker = docker.clone();
                move |id| {
                    docker
                        .containers()
                        .get(&id)
                        .logs(&LogsOptions::builder().stderr(true).follow(true).build())
                        .take_while(|chunk| {
                            let log = chunk.as_string_lossy();
                            Ok(!log.contains("Public node URL:"))
                        })
                        .collect()
                        .map(|_| id)
                }
            })
            .and_then({
                let http_url = http_url.clone();
                move |container_id| {
                    let (_event_loop, transport) = Http::new(&http_url).unwrap();
                    let http_client = Web3::new(transport);

                    Ok(EthereumNode {
                        container_id,
                        http_client,
                        _event_loop,
                    })
                }
            })
            .and_then({
                let envfile_path = envfile_path.clone();
                let http_url = http_url.clone();
                move |node| {
                    let mut envfile = EnvFile::new(envfile_path).unwrap();
                    envfile.update(&HTTP_URL_KEY, &http_url).write().unwrap();

                    Ok(node)
                }
            })
    }

    pub fn fund(
        &self,
        address: Address,
        value: U256,
    ) -> impl Future<Item = H256, Error = web3::error::Error> {
        let parity_dev_account: web3::types::Address =
            "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();

        self.http_client.personal().send_transaction(
            TransactionRequest {
                from: parity_dev_account,
                to: Some(address),
                gas: None,
                gas_price: None,
                value: Some(value),
                data: None,
                nonce: None,
                condition: None,
            },
            "",
        )
    }
}

impl Drop for EthereumNode {
    fn drop(&mut self) {
        let docker = Docker::new();

        let rm_fut = docker
            .containers()
            .get(&self.container_id)
            .remove(
                RmContainerOptions::builder()
                    .force(true)
                    .volumes(true)
                    .build(),
            )
            .map_err(|_| ());

        tokio::run(rm_fut);
    }
}

// Copied most of it from comit-rs/internal/key_gen
pub fn derive_address(secret_key: secp256k1::SecretKey) -> Address {
    let public_key =
        secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &secret_key);

    let serialized_public_key = public_key.serialize_uncompressed();
    // Remove the silly openssl 0x04 byte from the front of the
    // serialized public key. This is a bitcoin thing that
    // ethereum doesn't want. Eth pubkey should be 32 + 32 = 64 bytes.
    let actual_public_key = &serialized_public_key[1..];
    let hash = tiny_keccak::keccak256(actual_public_key);
    // Ethereum address is the last twenty bytes of the keccak256 hash
    let ethereum_address_bytes = &hash[12..];
    let mut address = Address::default();
    address.assign_from_slice(ethereum_address_bytes);
    address
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use web3::types::{Address, BlockId, BlockNumber, U128};

    #[test]
    fn can_ping_ethereum_node() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        let ethereum = runtime
            .block_on(EthereumNode::start(file.path().to_path_buf()))
            .unwrap();

        ethereum
            .http_client
            .eth()
            .block(BlockId::Number(BlockNumber::from(0)))
            .map(|block| assert_eq!(block.unwrap().number, Some(U128::from(0))))
            .wait()
            .unwrap();
    }

    #[test]
    fn can_fund_ethereum_address() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        let ethereum = runtime
            .block_on(EthereumNode::start(file.path().to_path_buf()))
            .unwrap();

        let address = Address::from_str("98e8183a8bf0b7805ed7eb1044ba3e9eb2ed6c1d").unwrap();
        let value = U256::from(1_000);

        let _ = runtime
            .block_on(ethereum.fund(address.clone().into(), value.clone().into()))
            .unwrap();

        let balance = runtime.block_on(ethereum.http_client.eth().balance(address.into(), None));
        assert_eq!(balance, Ok(value.into()));
    }

    #[test]
    fn can_get_http_port_from_envfile() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        runtime
            .block_on(EthereumNode::start(file.path().to_path_buf()))
            .unwrap();

        let envfile = EnvFile::new(&file.path()).unwrap();
        assert!(envfile.get(&HTTP_URL_KEY).is_some());
    }
}

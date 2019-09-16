use envfile::EnvFile;
use futures::stream::Stream;
use hdwallet::traits::Serialize;
use hdwallet::ExtendedPrivKey;
use shiplift::{ContainerOptions, Docker, LogsOptions, RmContainerOptions};
use std::path::PathBuf;
use web3::{
    api::Web3,
    futures::Future,
    transports::Http,
    types::{Address, TransactionRequest, U256},
};

const HTTP_PORT_KEY: &str = "ETHEREUM_NODE_HTTP_PORT";
const HD_KEY_KEY: &str = "ETHEREUM_HD_KEY";

pub struct EthereumNode {
    pub container_id: String,
    pub http_port: u32,
    pub hd_keys: Vec<ExtendedPrivKey>,
}

impl EthereumNode {
    pub fn start(envfile_path: PathBuf) -> impl Future<Item = Self, Error = ()> {
        let http_port: u32 = port_check::free_local_port().unwrap().into();

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
            .and_then(move |container_id| {
                let mut hd_keys = Vec::new();

                for _ in 0..2 {
                    let extended_private_key =
                        ExtendedPrivKey::random().expect("failed to generate extended private key");

                    hd_keys.push(extended_private_key);
                }

                Ok(EthereumNode {
                    container_id,
                    http_port,
                    hd_keys,
                })
            })
            .and_then({
                let envfile_path = envfile_path.clone();
                move |node| {
                    let mut envfile = EnvFile::new(envfile_path).unwrap();
                    envfile
                        .update(&HTTP_PORT_KEY, &http_port.to_string())
                        .write()
                        .unwrap();

                    Ok(node)
                }
            })
            // TODO: Improve error logging
            .map_err(|e| println!("Shiplift error: {}", e))
            .and_then(|node| {
                let mut envfile = EnvFile::new(envfile_path).unwrap();

                for (i, hd_key) in node.hd_keys.iter().enumerate() {
                    envfile.update(
                        format!("{}_{}", HD_KEY_KEY, i + 1).as_str(),
                        hex::encode(&hd_key.serialize()).as_str(),
                    );
                }

                envfile.write().unwrap();
                Ok(node)
            })
    }

    pub fn fund(&self, address: Address, value: U256) {
        let endpoint = format!("http://localhost:{}", &self.http_port);
        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let client = Web3::new(transport);

        let parity_dev_account: web3::types::Address =
            "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap();

        let fut = client
            .personal()
            .send_transaction(
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
            .map(|_| ())
            .map_err(|_| ());

        tokio::run(fut);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property_based::Quickcheck;
    use quickcheck;
    use web3::types::{Address, BlockId, BlockNumber, U128};

    #[test]
    fn can_ping_ethereum_node() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        let ethereum = runtime
            .block_on(EthereumNode::start(file.path().to_path_buf()))
            .unwrap();

        let endpoint = format!("http://localhost:{}", ethereum.http_port);
        let (_event_loop, transport) = Http::new(&endpoint).unwrap();
        let client = Web3::new(transport);

        client
            .eth()
            .block(BlockId::Number(BlockNumber::from(0)))
            .map(|block| assert_eq!(block.unwrap().number, Some(U128::from(0))))
            .wait()
            .unwrap();
    }

    #[test]
    fn can_fund_ethereum_address() {
        fn prop(address: Quickcheck<Address>, value: Quickcheck<U256>) -> bool {
            let mut runtime = tokio::runtime::Runtime::new().unwrap();

            let file = tempfile::Builder::new().tempfile().unwrap();

            let ethereum = runtime
                .block_on(EthereumNode::start(file.path().to_path_buf()))
                .unwrap();

            ethereum.fund(address.clone().into(), value.clone().into());

            let endpoint = format!("http://localhost:{}", ethereum.http_port);
            let (_event_loop, transport) = Http::new(&endpoint).unwrap();
            let client = Web3::new(transport);

            client
                .eth()
                .balance(address.into(), None)
                .map(|balance| balance == value.into())
                .wait()
                .unwrap()
        }

        quickcheck::QuickCheck::new()
            .max_tests(1)
            .quickcheck(prop as fn(Quickcheck<Address>, Quickcheck<U256>) -> bool)
    }

    #[test]
    fn can_get_http_port_from_envfile() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        runtime
            .block_on(EthereumNode::start(file.path().to_path_buf()))
            .unwrap();

        let envfile = EnvFile::new(&file.path()).unwrap();
        assert!(envfile.get(&HTTP_PORT_KEY).is_some());
    }

    #[test]
    fn can_get_two_bitcoin_hd_keys_from_envfile() {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let file = tempfile::Builder::new().tempfile().unwrap();

        runtime
            .block_on(EthereumNode::start(file.path().to_path_buf()))
            .unwrap();

        let envfile = EnvFile::new(&file.path()).unwrap();
        assert!(envfile.get(format!("{}_1", HD_KEY_KEY).as_str()).is_some());
        assert!(envfile.get(format!("{}_2", HD_KEY_KEY).as_str()).is_some())
    }
}

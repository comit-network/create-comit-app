use futures::stream::Stream;
use shiplift::{ContainerOptions, Docker, LogsOptions, RmContainerOptions};
use web3::{
    api::Web3,
    futures::Future,
    transports::Http,
    types::{Address, TransactionRequest, U256},
};

pub struct EthereumNode {
    pub container_id: String,
    pub http_port: u32,
}

impl EthereumNode {
    pub fn start() -> impl Future<Item = Self, Error = shiplift::errors::Error> {
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
                Ok(EthereumNode {
                    container_id,
                    http_port,
                })
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

        let ethereum = runtime.block_on(EthereumNode::start()).unwrap();

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

            let ethereum = runtime.block_on(EthereumNode::start()).unwrap();

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
}

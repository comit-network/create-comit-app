use crate::docker::bitcoin::{self, BitcoinNode};
use crate::docker::ethereum::{self, EthereumNode};
use crate::docker::{Node, NodeImage};
use crate::executable::btsieve::{self};
use crate::executable::cnd::{self};
use crate::executable::Executable;
use envfile::EnvFile;
use futures;
use futures::Future;
use hdwallet::traits::Serialize;
use hdwallet::{ExtendedPrivKey, KeyIndex};
use rust_bitcoin::Amount;
use std::path::PathBuf;
use tokio::runtime::Runtime;
use web3::types::U256;

const HTTP_PORT_BTSIEVE: &str = "HTTP_PORT_BTSIEVE";
const HTTP_PORT_CND: &str = "HTTP_PORT_CND";

// TODO: Ensure that the .env file can only be written to by only one process at a time
// TODO: Proper error handling in particular to allow for cleanup of state after a runtime error
// TODO: Improve logs
// TODO: Refactor to reduce code duplication

pub fn start_env() {
    let mut runtime = Runtime::new().unwrap();

    let envfile_path = PathBuf::from(".env");
    std::fs::File::create(envfile_path.clone()).expect("Could not create .env file");

    let bitcoin_node = start_and_fund_bitcoin_node(&runtime, &envfile_path);

    let ethereum_node = start_and_fund_ethereum_node(&runtime, &envfile_path);

    let results = runtime.block_on(bitcoin_node.join(ethereum_node)).unwrap();

    println!("Blockchain nodes up and running");

    let (bitcoin_hd_keys, ethereum_hd_keys) = ((results.0).1, (results.1).1);

    let mut envfile = EnvFile::new(envfile_path.clone()).unwrap();

    for (i, hd_key) in bitcoin_hd_keys.iter().enumerate() {
        envfile.update(
            format!("BITCOIN_HD_KEY_{}", i).as_str(),
            hex::encode(&hd_key.serialize()).as_str(),
        );
    }

    for (i, hd_key) in ethereum_hd_keys.iter().enumerate() {
        envfile.update(
            format!("ETHEREUM_HD_KEY_{}", i).as_str(),
            hex::encode(&hd_key.serialize()).as_str(),
        );
    }
    envfile.write().unwrap();

    start_btsieves(&mut runtime, &mut envfile);
    println!("Two btsieves up and running");

    start_cnds(&mut runtime, &mut envfile);
    println!("Two cnds up and running");

    // TODO: Unblocking this via CTRL+C doesn't call drop on the containers afterwards
    // TODO: Delete .env file at the end
    ::std::thread::park();
}

fn start_and_fund_bitcoin_node(
    runtime: &Runtime,
    envfile_path: &PathBuf,
) -> impl Future<Item = (Node<BitcoinNode>, Vec<ExtendedPrivKey>), Error = ()> {
    Node::<BitcoinNode>::start(envfile_path.clone())
        .and_then({
            let mut hd_keys = Vec::new();
            let executor = runtime.executor();
            move |node| {
                for _ in 0..2 {
                    hd_keys.push(ExtendedPrivKey::random().expect("failed to generate hd key"));
                }

                for hd_key in hd_keys.iter() {
                    let private_key = hd_key
                        .derive_private_key(KeyIndex::Normal(0))
                        .expect("failed to derive private key from hd key")
                        .private_key;
                    let address = bitcoin::derive_address(private_key);

                    executor.spawn(
                        node.node_image
                            .fund(address, Amount::from_sat(100_000_000))
                            .and_then(|_| Ok(()))
                            .map_err(|e| println!("Could not fund bitcoin address: {}", e)),
                    );
                }

                Ok((node, hd_keys))
            }
        })
        .map_err(|e| {
            println!("Bitcoin node error: {}", e);
        })
}

fn start_and_fund_ethereum_node(
    runtime: &Runtime,
    envfile_path: &PathBuf,
) -> impl Future<
    Item = (
        Node<EthereumNode>,
        Vec<hdwallet::extended_key::ExtendedPrivKey>,
    ),
    Error = (),
> {
    Node::<EthereumNode>::start(envfile_path.clone())
        .and_then({
            let mut hd_keys = Vec::new();
            let executor = runtime.executor();
            move |node| {
                for _ in 0..2 {
                    hd_keys.push(ExtendedPrivKey::random().expect("failed to generate hd key"));
                }

                for hd_key in hd_keys.iter() {
                    let private_key = hd_key
                        .derive_private_key(KeyIndex::Normal(0))
                        .expect("failed to derive private key from hd key")
                        .private_key;
                    let address = ethereum::derive_address(private_key);

                    executor.spawn(
                        node.node_image
                            .fund(address, U256::from("9000000000000000000"))
                            .and_then(|_| Ok(()))
                            .map_err(|e| println!("Could not fund ethereum addresses: {}", e)),
                    );
                }

                Ok((node, hd_keys))
            }
        })
        .map_err(|e| println!("Ethereum node error: {}", e))
}

fn start_btsieves(runtime: &mut Runtime, envfile: &mut EnvFile) {
    for i in 0..2 {
        let port_bind = port_check::free_local_port().unwrap();
        let settings = btsieve::Settings {
            http_api: btsieve::HttpApi {
                port_bind,
                ..Default::default()
            },
            bitcoin: Some(btsieve::Bitcoin {
                network: String::from("regtest"),
                node_url: String::from(
                    envfile
                        .get(bitcoin::HTTP_URL_KEY)
                        .expect("could not find var in envfile"),
                ),
            }),
            ethereum: Some(btsieve::Ethereum {
                node_url: String::from(
                    envfile
                        .get(ethereum::HTTP_URL_KEY)
                        .expect("could not find var in envfile"),
                ),
            }),
            ..Default::default()
        };

        envfile
            .update(
                format!("{}_{}", HTTP_PORT_BTSIEVE, i).as_str(),
                &settings.http_api.port_bind.to_string(),
            )
            .write()
            .unwrap();

        let btsieve = Executable::start("btsieve", settings);

        // May be better for btsieve to be a future which spawns a process,
        // waits for a second and then returns
        runtime.spawn(btsieve.future);

        // TODO: Should wait until btsieve logs
        // "warp drive engaged: listening on http://0.0.0.0:8181" instead
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

fn start_cnds(runtime: &mut Runtime, envfile: &mut EnvFile) {
    for i in 0..2 {
        let btsieve_port = envfile
            .get(format!("{}_{}", HTTP_PORT_BTSIEVE, i).as_str())
            .expect("could not find var in envfile");
        let btsieve_url = format!("http://localhost:{}", btsieve_port);

        let settings = cnd::Settings {
            btsieve: cnd::Btsieve {
                url: btsieve_url,
                ..Default::default()
            },
            ..Default::default()
        };

        envfile
            .update(
                format!("{}_{}", HTTP_PORT_CND, i).as_str(),
                &settings.http_api.port.to_string(),
            )
            .write()
            .unwrap();

        let cnd = Executable::start("cnd", settings);

        // May be better for cnd to be a future which spawns a process,
        // waits for a second and then returns
        runtime.spawn(cnd.future);

        // TODO: Should wait until cnd logs
        // "Starting HTTP server on V4(0.0.0.0:8000)" instead
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

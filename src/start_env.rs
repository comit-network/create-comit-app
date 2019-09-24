use crate::docker::bitcoin::{self, BitcoinNode};
use crate::docker::ethereum::{self, EthereumNode};
use crate::docker::{create_network, delete_network, BlockchainImage, Node};
use crate::executable::btsieve::{self, Btsieve};
use crate::executable::cnd::{self, Cnd};
use crate::executable::Executable;
use envfile::EnvFile;
use futures;
use futures::stream;
use futures::{Future, Stream};
use hdwallet::traits::Serialize;
use hdwallet::{ExtendedPrivKey, KeyIndex};
use rust_bitcoin::Amount;
use secp256k1::SecretKey;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::runtime::Runtime;
use web3::types::U256;

const HTTP_PORT_BTSIEVE: &str = "HTTP_PORT_BTSIEVE";
const HTTP_PORT_CND: &str = "HTTP_PORT_CND";

// TODO: Ensure that the .env file can only be written to by only one process at a time
// TODO: Proper error handling in particular to allow for cleanup of state after a runtime error
// TODO: Improve logs
// TODO: Refactor to reduce code duplication

macro_rules! print_progress {
    ($($arg:tt)*) => ({
        print!($($arg)*);
        print!("...");
        std::io::stdout().flush().ok().expect("Could not flush stdout");
    })
}

pub fn start_env() {
    let mut bitcoin_hd_keys = vec![];
    let mut ethereum_hd_keys = vec![];
    for _ in 0..2 {
        bitcoin_hd_keys.push(ExtendedPrivKey::random().expect("Could not generate HD key"));
        ethereum_hd_keys.push(ExtendedPrivKey::random().expect("Could not generate HD key"));
    }

    let mut bitcoin_priv_keys = vec![];
    for hd_key in &bitcoin_hd_keys {
        bitcoin_priv_keys.push(derive_key(hd_key).expect("Could not derive keys"));
    }

    let mut ethereum_priv_keys = vec![];
    for hd_key in &ethereum_hd_keys {
        ethereum_priv_keys.push(derive_key(hd_key).expect("Could not derive keys"));
    }

    let envfile_path = PathBuf::from(".env");
    std::fs::File::create(envfile_path.clone()).expect("Could not create .env file");

    let docker_network_create = create_network();

    let bitcoin_node = start_bitcoin_node(&envfile_path, bitcoin_priv_keys).map_err(|e| {
        eprintln!("Issue starting Bitcoin node: {:?}", e);
    });

    let ethereum_node =
        start_ethereum_node(&envfile_path, ethereum_priv_keys.clone()).map_err(|e| {
            eprintln!("Issue starting Ethereum node: {:?}", e);
        });

    let mut runtime = Runtime::new().unwrap();

    print_progress!("Creating Docker network (create-comit-app)");
    let docker_network_id = runtime
        .block_on(docker_network_create)
        .map_err({
            let envfile_path = envfile_path.clone();
            |e| {
                eprintln!("Could not create docker network, cleaning up...\n{:?}", e);
                clean_up(&mut runtime, envfile_path, None, None);
            }
        })
        .unwrap();
    println!("✓");

    // TODO: use await to avoid all these clones

    print_progress!("Starting Bitcoin node");
    let bitcoin_node = runtime
        .block_on(bitcoin_node)
        .map_err({
            let envfile_path = envfile_path.clone();
            |e| {
                eprintln!("Could not start bitcoin node, cleaning up...\n{:?}", e);
                // TODO: The clean up should also try to delete the bitcoin container if it exists
                clean_up(&mut runtime, envfile_path, None, None);
            }
        })
        .map(Arc::new)
        .unwrap();
    println!("✓");

    print_progress!("Starting Ethereum node");
    let ethereum_node = runtime
        .block_on(ethereum_node)
        .map_err({
            let envfile_path = envfile_path.clone();
            let bitcoin_node = bitcoin_node.clone();

            |e| {
                eprintln!("Could not start Ethereum node, cleaning up...\n{:?}", e);
                clean_up(&mut runtime, envfile_path, Some(bitcoin_node), None);
            }
        })
        .map(Arc::new)
        .unwrap();
    println!("✓");

    print_progress!("Writing configuration in `.env` file");
    let mut envfile = EnvFile::new(envfile_path.clone())
        .map_err({
            let envfile_path = envfile_path.clone();
            let bitcoin_node = bitcoin_node.clone();
            let ethereum_node = ethereum_node.clone();

            |e| {
                eprintln!("Could not read .env file, cleaning up...\n{:?}", e);
                clean_up(
                    &mut runtime,
                    envfile_path,
                    Some(bitcoin_node),
                    Some(ethereum_node),
                );
            }
        })
        .unwrap();

    for (i, hd_key) in bitcoin_hd_keys.iter().enumerate() {
        envfile.update(
            format!("BITCOIN_HD_KEY_{}", i).as_str(),
            hex::encode(&hd_key.serialize()).as_str(),
        );
    }

    for (i, priv_key) in ethereum_priv_keys.iter().enumerate() {
        envfile.update(
            format!("ETHEREUM_KEY_{}", i).as_str(),
            format!("{}", priv_key).as_str(),
        );
    }

    envfile
        .write()
        .map_err({
            let envfile_path = envfile_path.clone();
            let bitcoin_node = bitcoin_node.clone();
            let ethereum_node = ethereum_node.clone();

            |e| {
                eprintln!("Could not write .env file, cleaning up...\n{:?}", e);
                clean_up(
                    &mut runtime,
                    envfile_path,
                    Some(bitcoin_node),
                    Some(ethereum_node),
                );
            }
        })
        .unwrap();
    println!("✓");

    print_progress!("Starting two btsieves");
    start_btsieves(&mut runtime, &mut envfile);
    println!("✓");

    print_progress!("Starting two cnds");
    start_cnds(&mut runtime, &mut envfile);
    println!("✓");

    println!("🎉 Environment is ready, time to create a COMIT app!");
    handle_signal(
        &mut runtime,
        envfile_path,
        bitcoin_node,
        ethereum_node,
        docker_network_id,
    );
}

#[derive(Debug)]
enum Error {
    HdKey(hdwallet::error::Error),
    BitcoinFunding(bitcoincore_rpc::Error),
    EtherFunding(web3::Error),
    Docker(shiplift::Error),
}

fn derive_key(hd_key: &ExtendedPrivKey) -> Result<SecretKey, Error> {
    Ok(hd_key
        .derive_private_key(KeyIndex::Normal(0))
        .map_err(Error::HdKey)?
        .private_key)
}

fn start_bitcoin_node(
    envfile_path: &PathBuf,
    secret_keys: Vec<SecretKey>,
) -> impl Future<Item = Node<BitcoinNode>, Error = Error> {
    Node::<BitcoinNode>::start(envfile_path.clone())
        .map_err(Error::Docker)
        .and_then(move |node| {
            stream::iter_ok(secret_keys).fold(node, |node, key| {
                node.node_image
                    .fund(bitcoin::derive_address(key), Amount::from_sat(100_000_000))
                    .map_err(Error::BitcoinFunding)
                    .map(|_| node)
            })
        })
}

fn start_ethereum_node(
    envfile_path: &PathBuf,
    secret_keys: Vec<SecretKey>,
) -> impl Future<Item = Node<EthereumNode>, Error = Error> {
    Node::<EthereumNode>::start(envfile_path.clone())
        .map_err(Error::Docker)
        .and_then(move |node| {
            stream::iter_ok(secret_keys).fold(node, |node, key| {
                node.node_image
                    .fund(
                        ethereum::derive_address(key),
                        U256::from("9000000000000000000"),
                    )
                    .map_err(Error::EtherFunding)
                    .map(|_| node)
            })
        })
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

        let btsieve = Executable::start::<Btsieve, _>(settings);

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

        let cnd = Executable::start::<Cnd, _>(settings);

        // May be better for cnd to be a future which spawns a process,
        // waits for a second and then returns
        runtime.spawn(cnd.future);

        // TODO: Should wait until cnd logs
        // "Starting HTTP server on V4(0.0.0.0:8000)" instead
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

fn handle_signal(
    runtime: &mut Runtime,
    envfile_path: PathBuf,
    bitcoin_node: Arc<Node<BitcoinNode>>,
    ethereum_node: Arc<Node<EthereumNode>>,
    docker_network_id: String,
) {
    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&terminate))
        .expect("Could not register SIGTERM");
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&terminate))
        .expect("Could not register SIGINT");
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&terminate))
        .expect("Could not register SIGQUIT");
    while !terminate.load(Ordering::Relaxed) {
        sleep(Duration::from_millis(50))
    }
    println!("Signal received, terminating...");
    clean_up(
        runtime,
        envfile_path,
        Some(bitcoin_node),
        Some(ethereum_node),
    );
    clean_up_docker_network(runtime, docker_network_id)
}

// TODO: Split this method, return futures
fn clean_up(
    runtime: &mut Runtime,
    envfile_path: PathBuf,
    bitcoin_node: Option<Arc<Node<BitcoinNode>>>,
    ethereum_node: Option<Arc<Node<EthereumNode>>>,
) {
    if let Some(bitcoin_node) = bitcoin_node {
        let _ = runtime
            .block_on(bitcoin_node.stop_remove())
            .map_err(|e| eprintln!("Runtime could not run bitcoin docker terminate: {:?}", e));
    };
    if let Some(ethereum_node) = ethereum_node {
        let _ = runtime
            .block_on(ethereum_node.stop_remove())
            .map_err(|e| eprintln!("Runtime could not run ethereum docker terminate: {:?}", e));
    };
    let _ = std::fs::remove_file(envfile_path)
        .map_err(|e| eprintln!("Could not remove .env file: {:?}", e));
}

fn clean_up_docker_network(runtime: &mut Runtime, id: String) {
    let _ = runtime
        .block_on(delete_network(id))
        .map_err(|e| eprintln!("Runtime could not delete docker network: {:?}", e));
}

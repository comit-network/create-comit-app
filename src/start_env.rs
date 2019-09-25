use crate::comit_rs_settings::btsieve::{self};
use crate::comit_rs_settings::cnd::{self};
use crate::docker::bitcoin::{self, BitcoinNode};
use crate::docker::ethereum::{self, EthereumNode};
use crate::docker::Btsieve;
use crate::docker::Cnd;
use crate::docker::{create_network, delete_network, BlockchainImage, Node};
use envfile::EnvFile;
use futures;
use futures::stream;
use futures::{Future, Stream};
use hdwallet::traits::Serialize;
use hdwallet::{ExtendedPrivKey, KeyIndex};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rust_bitcoin::Amount;
use secp256k1::SecretKey;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::runtime::Runtime;
use web3::types::U256;

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

    let btsieves = start_btsieves(&envfile_path).map_err(|e| {
        eprintln!("Issue starting btsieves: {:?}", e);
    });

    let cnds = start_cnds(&envfile_path).map_err(|e| {
        eprintln!("Issue starting cnds: {:?}", e);
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
    let btsieves = runtime
        .block_on(btsieves)
        .map_err({
            let envfile_path = envfile_path.clone();
            let bitcoin_node = bitcoin_node.clone();
            let ethereum_node = ethereum_node.clone();

            |e| {
                eprintln!("Could not start btsieves, cleaning up...\n{:?}", e);
                clean_up(
                    &mut runtime,
                    envfile_path,
                    Some(bitcoin_node),
                    Some(ethereum_node),
                );
            }
        })
        .map(Arc::new)
        .unwrap();
    println!("✓");

    print_progress!("Starting two cnds");
    let cnds = runtime
        .block_on(cnds)
        .map_err({
            let envfile_path = envfile_path.clone();
            let bitcoin_node = bitcoin_node.clone();
            let ethereum_node = ethereum_node.clone();

            |e| {
                eprintln!("Could not start cnds, cleaning up...\n{:?}", e);
                clean_up(
                    &mut runtime,
                    envfile_path,
                    Some(bitcoin_node),
                    Some(ethereum_node),
                );
            }
        })
        .map(Arc::new)
        .unwrap();
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
    CreateDir(std::io::Error),
    WriteConfig(std::io::Error),
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
    Node::<BitcoinNode>::start(envfile_path.clone(), "bitcoin")
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
    Node::<EthereumNode>::start(envfile_path.clone(), "ethereum")
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

fn start_btsieves(envfile_path: &PathBuf) -> impl Future<Item = Vec<Node<Btsieve>>, Error = Error> {
    let settings = btsieve::Settings {
        http_api: btsieve::HttpApi {
            port_bind: 8080,
            ..Default::default()
        },
        bitcoin: Some(btsieve::Bitcoin {
            network: String::from("regtest"),
            node_url: "http://bitcoin:18443".to_string(),
        }),
        ethereum: Some(btsieve::Ethereum {
            node_url: "http://ethereum:8545".to_string(),
        }),
        ..Default::default()
    };

    let config_folder = temp_folder();
    let config_file = config_folder.clone().join("btsieve.toml");

    let settings = toml::to_string(&settings).expect("could not serialize settings");
    let volume = format!("{}:/config", config_folder.clone().to_str().unwrap());

    // TODO: delete folder at clean up
    tokio::fs::create_dir_all(config_folder.clone())
        .map_err(Error::CreateDir)
        .and_then(|_| tokio::fs::write(config_file, settings).map_err(Error::WriteConfig))
        .and_then({
            let envfile_path = envfile_path.clone();
            move |_| {
                stream::iter_ok(vec![0, 1])
                    .and_then({
                        let volume = volume.clone();
                        let envfile_path = envfile_path.clone();
                        move |i| {
                            Node::<Btsieve>::start_with_volume(
                                envfile_path.to_path_buf(),
                                format!("btsieve_{}", i).as_str(),
                                &volume,
                            )
                            .map_err(Error::Docker)
                        }
                    })
                    .collect()
            }
        })
}

fn start_cnds(envfile_path: &PathBuf) -> impl Future<Item = Vec<Node<Cnd>>, Error = Error> {
    stream::iter_ok(vec![0, 1])
        .and_then({
            let envfile_path = envfile_path.clone();

            move |i| {
                let config_folder = temp_folder();

                tokio::fs::create_dir_all(config_folder.clone())
                    .map_err(Error::CreateDir)
                    .and_then({
                        let config_folder = config_folder.clone();

                        move |_| {
                            let settings = cnd::Settings {
                                btsieve: cnd::Btsieve {
                                    url: format!("http://btsieve_{}:8080", i),
                                    ..Default::default()
                                },
                                ..Default::default()
                            };

                            let config_file = config_folder.join("cnd.toml");
                            let settings =
                                toml::to_string(&settings).expect("could not serialize settings");

                            tokio::fs::write(config_file, settings).map_err(Error::WriteConfig)
                        }
                    })
                    .and_then({
                        let config_folder = config_folder.clone();
                        let envfile_path = envfile_path.clone();

                        move |_| {
                            let volume = format!("{}:/config", config_folder.to_str().unwrap());

                            Node::<Cnd>::start_with_volume(
                                envfile_path.to_path_buf(),
                                format!("cnd_{}", i).as_str(),
                                &volume,
                            )
                            .map_err(Error::Docker)
                        }
                    })
            }
        })
        .collect()
}

fn temp_folder() -> PathBuf {
    let rng = thread_rng();
    let folder_name: String = rng.sample_iter(Alphanumeric).take(7).collect();
    // Default $TMPDIR in Mac OS is under /var/folders/
    // However this is not a "shared path" by default for Docker
    // (see Docker > Preferences)
    #[cfg(target_os = "macos")]
    let mut config_folder = PathBuf::from_str("/tmp/create-comit-app").expect("Infaillible");
    #[cfg(not(target_os = "macos"))]
    let mut config_folder = std::env::temp_dir();
    config_folder.push(folder_name);
    config_folder
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

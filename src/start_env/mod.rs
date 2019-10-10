use crate::cnd_settings;
use crate::docker::bitcoin::{self, BitcoinNode};
use crate::docker::delete_container;
use crate::docker::ethereum::{self, EthereumNode};
use crate::docker::Cnd;
use crate::docker::{create_network, delete_network, Node};
use crate::print_progress;
use bitcoincore_rpc::RpcApi;
use envfile::EnvFile;
use futures;
use futures::stream;
use futures::{Future, Stream};
use rand::{thread_rng, Rng};
use rust_bitcoin::util::bip32::ChildNumber;
use rust_bitcoin::util::bip32::ExtendedPrivKey;
use rust_bitcoin::Amount;
use secp256k1::{Secp256k1, SecretKey};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::timer::Interval;
use web3::types::U256;

mod temp_fs;

pub fn start_env() {
    let mut runtime = Runtime::new().expect("Could not get runtime");

    if temp_fs::dir_exist() {
        eprintln!("It seems that `create-comit-app start-env` is already running.\nIf it is not the case, delete lock directory ~/{} and try again.", temp_fs::DIR_NAME);
        ::std::process::exit(1);
    }

    let terminate = register_signals().expect("Could not register signals");

    match start_all(&mut runtime, &terminate) {
        Ok(Services { bitcoin_node, .. }) => {
            runtime.spawn(bitcoin_generate_blocks(bitcoin_node.clone()));

            runtime
                .block_on(handle_signal(terminate))
                .expect("Handle signal failed");
            println!("✓");
        }
        Err(err) => {
            match &err {
                Error::SignalReceived => {
                    println!("Signal received, terminating...");
                }
                _ => {
                    eprintln!("❗️Error encountered: {:?}]", err);
                }
            }

            print_progress!("🧹 Cleaning up");
            runtime.block_on(clean_up()).expect("Clean up failed");
            println!("✓");
        }
    }
}

fn bitcoin_generate_blocks(
    bitcoin_node: Arc<Node<BitcoinNode>>,
) -> impl Future<Item = (), Error = ()> {
    Interval::new_interval(Duration::from_secs(1))
        .map_err(|_| eprintln!("Issue getting an interval."))
        .for_each({
            let bitcoin_node = bitcoin_node.clone();
            move |_| {
                let _ = bitcoin_node.node_image.rpc_client.generate(1, None);
                Ok(())
            }
        })
}

#[allow(dead_code)]
struct Services {
    docker_network_id: String,
    bitcoin_node: Arc<Node<BitcoinNode>>,
    ethereum_node: Arc<Node<EthereumNode>>,
    cnds: Arc<Vec<Node<Cnd>>>,
}

fn check_signal(terminate: &Arc<AtomicBool>) -> Result<(), Error> {
    if terminate.load(Ordering::Relaxed) {
        Err(Error::SignalReceived)
    } else {
        Ok(())
    }
}

fn start_all(runtime: &mut Runtime, terminate: &Arc<AtomicBool>) -> Result<Services, Error> {
    let (
        bitcoin_hd_keys,
        ethereum_priv_keys,
        env_file_path,
        docker_network_create,
        bitcoin_node,
        ethereum_node,
        cnds,
    ) = all_futures()?;

    let env_file_str = temp_fs::create_env_file()?;

    print_progress!("Creating Docker network (create-comit-app)");
    let docker_network_id = runtime.block_on(docker_network_create).map_err(|e| {
        eprintln!("Could not create docker network, aborting...\n{:?}", e);
    })?;
    println!("✓");
    check_signal(terminate)?;

    print_progress!("Starting Bitcoin node");
    let bitcoin_node = runtime
        .block_on(bitcoin_node)
        .map_err(|e| {
            eprintln!("Could not start bitcoin node, aborting...\n{:?}", e);
        })
        .map(Arc::new)?;
    println!("✓");
    check_signal(terminate)?;

    print_progress!("Starting Ethereum node");
    let ethereum_node = runtime
        .block_on(ethereum_node)
        .map_err(|e| {
            eprintln!("Could not start Ethereum node, aborting...\n{:?}", e);
        })
        .map(Arc::new)?;
    println!("✓");
    check_signal(terminate)?;

    print_progress!("Writing configuration in env file");
    let mut envfile = EnvFile::new(env_file_path.clone()).map_err(|e| {
        eprintln!("Could not read {} file, aborting...\n{:?}", env_file_str, e);
    })?;

    for (i, hd_key) in bitcoin_hd_keys.iter().enumerate() {
        envfile.update(
            format!("BITCOIN_HD_KEY_{}", i).as_str(),
            format!("{}", hd_key).as_str(),
        );
    }

    for (i, priv_key) in ethereum_priv_keys.iter().enumerate() {
        envfile.update(
            format!("ETHEREUM_KEY_{}", i).as_str(),
            format!("{}", priv_key).as_str(),
        );
    }

    envfile.write().map_err(|e| {
        eprintln!(
            "Could not write {} file, aborting...\n{:?}",
            env_file_str, e
        );
    })?;
    println!("✓");
    check_signal(terminate)?;

    print_progress!("Starting two cnds");
    let cnds = runtime
        .block_on(cnds)
        .map_err(|e| {
            eprintln!("Could not start cnds, cleaning up...\n{:?}", e);
        })
        .map(Arc::new)?;
    println!("✓");
    check_signal(terminate)?;

    println!("🎉 Environment is ready, time to create a COMIT app!");
    Ok(Services {
        docker_network_id,
        bitcoin_node,
        ethereum_node,
        cnds,
    })
}

fn all_futures() -> Result<
    (
        Vec<ExtendedPrivKey>,
        Vec<SecretKey>,
        PathBuf,
        impl Future<Item = String, Error = ()>,
        impl Future<Item = Node<BitcoinNode>, Error = ()>,
        impl Future<Item = Node<EthereumNode>, Error = ()>,
        impl Future<Item = Vec<Node<Cnd>>, Error = ()>,
    ),
    Error,
> {
    let bitcoin_hd_keys = vec![
        new_extended_regtest_priv_key()?,
        new_extended_regtest_priv_key()?,
    ];
    let derivation_path = vec![
        ChildNumber::from_hardened_idx(44)?,
        ChildNumber::from_hardened_idx(1)?,
        ChildNumber::from_hardened_idx(0)?,
        ChildNumber::from_normal_idx(0)?,
        ChildNumber::from_normal_idx(0)?,
    ];
    let bitcoin_priv_keys = bitcoin_hd_keys
        .iter()
        .map(|hd_key| {
            hd_key
                .derive_priv(&Secp256k1::new(), &derivation_path)
                .map(|secret_key| secret_key.private_key.key)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ethereum_priv_keys = vec![
        SecretKey::new(&mut thread_rng()),
        SecretKey::new(&mut thread_rng()),
    ];

    let env_file_path = temp_fs::env_file_path()?;
    let docker_network_create = create_network().map_err(|e| {
        eprintln!("Issue creating Docker network: {:?}", e);
    });
    let bitcoin_node = start_bitcoin_node(&env_file_path, bitcoin_priv_keys).map_err(|e| {
        eprintln!("Issue starting Bitcoin node: {:?}", e);
    });
    let ethereum_node =
        start_ethereum_node(&env_file_path, ethereum_priv_keys.clone()).map_err(|e| {
            eprintln!("Issue starting Ethereum node: {:?}", e);
        });
    let cnds = {
        let (path, string) = temp_fs::temp_folder()?;
        start_cnds(&env_file_path, path, string).map_err(|e| {
            eprintln!("Issue starting cnds: {:?}", e);
        })
    };
    Ok((
        bitcoin_hd_keys,
        ethereum_priv_keys,
        env_file_path,
        docker_network_create,
        bitcoin_node,
        ethereum_node,
        cnds,
    ))
}

fn new_extended_regtest_priv_key() -> Result<ExtendedPrivKey, rust_bitcoin::util::bip32::Error> {
    ExtendedPrivKey::new_master(rust_bitcoin::Network::Regtest, &{
        let mut seed = [0u8; 32];
        thread_rng().fill_bytes(&mut seed);

        seed
    })
}

#[derive(Debug)]
pub enum Error {
    BitcoinFunding(bitcoincore_rpc::Error),
    EtherFunding(web3::Error),
    Docker(shiplift::Error),
    CreateTmpFiles(std::io::Error),
    PathToStr,
    WriteConfig(std::io::Error),
    DeriveKeys(rust_bitcoin::util::bip32::Error),
    HomeDir,
    SignalReceived,
    Unimplemented,
}

fn start_bitcoin_node(
    envfile_path: &PathBuf,
    secret_keys: Vec<SecretKey>,
) -> impl Future<Item = Node<BitcoinNode>, Error = Error> {
    Node::<BitcoinNode>::start(envfile_path.clone(), "bitcoin")
        .map_err(Error::Docker)
        .and_then(move |node| {
            stream::iter_ok(secret_keys).fold(node, |node, key| {
                bitcoin::fund(
                    &node.node_image.rpc_client,
                    bitcoin::derive_address(key),
                    Amount::from_sat(1_000_000_000),
                )
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
                ethereum::fund(
                    &node.node_image.http_client,
                    ethereum::derive_address(key),
                    U256::from(100u128 * 10u128.pow(18)),
                )
                .map_err(Error::EtherFunding)
                .map(|_| node)
            })
        })
}

//fn deploy_er20_and_mint_tokens(node: Node<EthereumNode>, keys: impl Iterator<Item = SecretKey>) -> impl Future {
//    let client = node.node_image.http_client;
//
//    client.eth()
//}

fn start_cnds(
    envfile_path: &PathBuf,
    config_folder: PathBuf,
    config_folder_str: String,
) -> impl Future<Item = Vec<Node<Cnd>>, Error = Error> {
    stream::iter_ok(vec![0, 1])
        .and_then({
            let envfile_path = envfile_path.clone();

            move |i| {
                tokio::fs::create_dir_all(config_folder.clone())
                    .map_err(Error::CreateTmpFiles)
                    .and_then({
                        let config_folder = config_folder.clone();

                        move |_| {
                            let settings = cnd_settings::Settings {
                                bitcoin: cnd_settings::Bitcoin {
                                    network: String::from("regtest"),
                                    node_url: "http://bitcoin:18443".to_string(),
                                },
                                ethereum: cnd_settings::Ethereum {
                                    network: String::from("regtest"),
                                    node_url: "http://ethereum:8545".to_string(),
                                },
                                ..Default::default()
                            };

                            let config_file = config_folder.join("cnd.toml");
                            let settings = toml::to_string(&settings)
                                .expect("could not serialize hardcoded settings");

                            tokio::fs::write(config_file, settings).map_err(Error::WriteConfig)
                        }
                    })
                    .and_then({
                        let envfile_path = envfile_path.clone();
                        let config_folder_str = config_folder_str.clone();
                        move |_| {
                            let volume = format!("{}:/config", config_folder_str);

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

fn handle_signal(terminate: Arc<AtomicBool>) -> impl Future<Item = (), Error = ()> {
    while !terminate.load(Ordering::Relaxed) {
        sleep(Duration::from_millis(50))
    }
    println!("Signal received, terminating...");
    print_progress!("🧹 Cleaning up");
    clean_up()
}

fn register_signals() -> Result<Arc<AtomicBool>, std::io::Error> {
    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&terminate))?;
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&terminate))?;
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&terminate))?;
    Ok(terminate)
}

fn clean_up() -> impl Future<Item = (), Error = ()> {
    delete_container("bitcoin")
        .then(|_| delete_container("ethereum"))
        .then(|_| {
            stream::iter_ok(vec![0, 1])
                .and_then(move |i| delete_container(format!("cnd_{}", i).as_str()))
                .collect()
        })
        .then(|_| delete_network())
        .then(|_| {
            let _ = temp_fs::dir_path().map(std::fs::remove_dir_all);
            Ok(())
        })
        .map_err(|_: ()| ())
}

impl From<()> for Error {
    fn from(_: ()) -> Self {
        Error::Unimplemented
    }
}

impl From<rust_bitcoin::util::bip32::Error> for Error {
    fn from(err: rust_bitcoin::util::bip32::Error) -> Self {
        Error::DeriveKeys(err)
    }
}

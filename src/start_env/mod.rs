use crate::docker::bitcoin::{BitcoinNode, GenerateQuery};
use crate::docker::delete_container;
use crate::docker::{delete_network, Node};
use crate::print_progress;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::prelude::stream;
use tokio::prelude::{Future, Stream};
use tokio::runtime::Runtime;
use tokio::timer::Interval;

mod start;
mod temp_fs;

pub fn start_env() {
    let mut runtime = Runtime::new().expect("Could not get runtime");

    if temp_fs::dir_exist() {
        eprintln!("It seems that `create-comit-app start-env` is already running.\nIf it is not the case, delete lock directory ~/{} and try again.", temp_fs::DIR_NAME);
        ::std::process::exit(1);
    }

    let terminate = register_signals().expect("Could not register signals");

    match self::start::start_all(&mut runtime, &terminate) {
        Ok(self::start::Services { bitcoin_node, .. }) => {
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
            let generate_req = GenerateQuery::new(1);
            move |_| {
                reqwest::r#async::Client::new()
                    .post(&bitcoin_node.node_image.endpoint)
                    .basic_auth(
                        &bitcoin_node.node_image.username,
                        Some(&bitcoin_node.node_image.password),
                    )
                    .json(&generate_req)
                    .send()
                    .map(|_| ())
                    .map_err(|err| {
                        eprintln!(
                            "Error encountered when generating bitcoin blocks: {:?}",
                            err
                        )
                    })
            }
        })
}

#[derive(Debug)]
pub enum Error {
    BitcoinFunding(reqwest::Error),
    EtherFunding(web3::Error),
    Erc20Funding(web3::Error),
    Docker(shiplift::Error),
    CreateTmpFiles(std::io::Error),
    PathToStr,
    WriteConfig(std::io::Error),
    DeriveKeys(rust_bitcoin::util::bip32::Error),
    HomeDir,
    SignalReceived,
    Unimplemented,
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

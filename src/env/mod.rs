use crate::docker::bitcoin::{BitcoinNode, GenerateQuery};
use crate::docker::Node;
use crate::print_progress;
use std::sync::Arc;
use std::time::Duration;
use tokio::prelude::{Future, Stream};
use tokio::runtime::Runtime;
use tokio::timer::Interval;

mod clean_up;
mod start;
mod temp_fs;

pub fn start() {
    let mut runtime = Runtime::new().expect("Could not get runtime");

    if temp_fs::dir_exist() {
        eprintln!("It seems that `create-comit-app start-env` is already running.\nIf it is not the case, delete lock directory ~/{} and try again.", temp_fs::DIR_NAME);
        ::std::process::exit(1);
    }

    let terminate = self::clean_up::register_signals().expect("Could not register signals");

    std::panic::set_hook(Box::new(|panic_info| {
        print_progress!("Panic received, cleaning up");
        tokio::runtime::current_thread::block_on_all(self::clean_up::clean_up())
            .expect("Clean up failed");
        println!("✓");
        eprintln!("{}", panic_info);
    }));

    match self::start::execute(&mut runtime, &terminate) {
        Ok(self::start::Services { bitcoin_node, .. }) => {
            runtime.spawn(bitcoin_generate_blocks(bitcoin_node.clone()));

            runtime
                .block_on(self::clean_up::handle_signal(terminate))
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
            runtime
                .block_on(self::clean_up::clean_up())
                .expect("Clean up failed");
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

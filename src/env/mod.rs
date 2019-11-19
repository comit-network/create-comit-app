use crate::{
    docker::{
        bitcoin::{BitcoinNode, GenerateQuery},
        Node,
    },
    env::start::SignalReceived,
    print_progress,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    prelude::{Future, Stream},
    runtime::Runtime,
    timer::Interval,
};

mod clean_up;
mod start;
mod temp_fs;

pub fn clean_up() {
    tokio::runtime::current_thread::block_on_all(self::clean_up::clean_up())
        .expect("Clean up failed");
    println!("Clean up done!");
}

pub fn start() {
    let mut runtime = Runtime::new().expect("Could not get runtime");

    if temp_fs::dir_exist() {
        eprintln!("It seems that `create-comit-app start-env` is already running.\nIf it is not the case, run `create-comit-app force-clean-env` and try again.");
        ::std::process::exit(1);
    }

    let terminate = self::clean_up::register_signals().expect("Could not register signals");

    std::panic::set_hook(Box::new(|panic_info| {
        print_progress!("Panic received, cleaning up");
        clean_up();
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
            if err.downcast_ref::<SignalReceived>().is_some() {
                println!("Signal received, terminating...");
            } else {
                eprintln!("❗️Error encountered: {:?}]", err);
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

use crate::{
    docker::bitcoin::{self, BitcoindHttpEndpoint},
    env::start::SignalReceived,
    print_progress,
};
use futures::{compat::Future01CompatExt, FutureExt, TryFutureExt};
use std::{
    ops::Add,
    time::{Duration, Instant},
};
use tokio::{runtime::Runtime, timer::Delay};

mod clean_up;
mod start;

pub fn clean_up() {
    tokio::runtime::current_thread::block_on_all(self::clean_up::clean_up())
        .expect("Clean up failed");
    println!("Clean up done!");
}

pub fn start() {
    let mut runtime = Runtime::new().expect("Could not get runtime");

    if crate::temp_fs::dir_exist() {
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

    match runtime.block_on(self::start::execute(terminate.clone()).boxed().compat()) {
        Ok(self::start::Environment { bitcoind, .. }) => {
            let miner = new_miner(bitcoind.http_endpoint)
                .map_err(|_| ())
                .boxed()
                .compat();

            runtime.spawn(miner);
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

async fn new_miner(endpoint: BitcoindHttpEndpoint) -> anyhow::Result<()> {
    loop {
        Delay::new(Instant::now().add(Duration::from_secs(1)))
            .compat()
            .await?;
        bitcoin::mine_a_block(endpoint).await?;
    }
}

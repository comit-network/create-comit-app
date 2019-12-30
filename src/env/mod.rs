use crate::{
    docker::bitcoin::{self, BitcoindHttpEndpoint},
    env::start::SignalReceived,
    print_progress,
};
use std::time::Duration;
use tokio::time::delay_for;

pub use clean_up::clean_up;

mod clean_up;
mod start;

pub async fn start() {
    if crate::temp_fs::dir_exist() {
        eprintln!("It seems that `create-comit-app start-env` is already running.\nIf it is not the case, run `create-comit-app force-clean-env` and try again.");
        ::std::process::exit(1);
    }

    let terminate = self::clean_up::register_signals().expect("Could not register signals");

    match self::start::execute(terminate.clone()).await {
        Ok(self::start::Environment { bitcoind, .. }) => {
            tokio::spawn(new_miner(bitcoind.http_endpoint));

            self::clean_up::handle_signal(terminate).await;

            println!("✓");
        }
        Err(err) => {
            if err.downcast_ref::<SignalReceived>().is_some() {
                println!("Signal received, terminating...");
            } else {
                eprintln!("❗️Error encountered: {:?}]", err);
            }

            print_progress!("🧹 Cleaning up");
            self::clean_up::clean_up().await;
            println!("✓");
        }
    }
}

async fn new_miner(endpoint: BitcoindHttpEndpoint) -> anyhow::Result<()> {
    loop {
        delay_for(Duration::from_secs(1)).await;
        bitcoin::mine_a_block(endpoint).await?;
    }
}

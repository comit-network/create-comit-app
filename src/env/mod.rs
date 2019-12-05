use crate::{
    docker::bitcoin::{self, BitcoindHttpEndpoint},
    env::start::SignalReceived,
    print_progress,
};
use std::time::Duration;

mod clean_up;
mod start;

pub use self::clean_up::clean_up;

pub async fn start() {
    if crate::temp_fs::dir_exist().await {
        eprintln!("It seems that `create-comit-app start-env` is already running.\nIf it is not the case, run `create-comit-app force-clean-env` and try again.");
        ::std::process::exit(1);
    }

    let terminate = self::clean_up::register_signals().expect("Could not register signals");

    std::panic::set_hook(Box::new(move |panic_info| {
        print_progress!("Panic received, cleaning up");

        tokio::spawn(self::clean_up::clean_up());

        println!("✓");
        eprintln!("{}", panic_info);
    }));

    let result = self::start::execute(terminate.clone()).await;

    match result {
        Ok(environment) => {
            tokio::spawn(new_miner(environment.bitcoind.http_endpoint));

            self::clean_up::handle_signal(terminate).await;
        }
        Err(err) => {
            if err.downcast_ref::<SignalReceived>().is_some() {
                println!("Signal received, terminating...");
            } else {
                eprintln!("❗️Error encountered: {:?}]", err);
            }
        }
    }

    print_progress!("🧹 Cleaning up");
    self::clean_up::clean_up().await;
    println!("✓");
}

async fn new_miner(endpoint: BitcoindHttpEndpoint) {
    loop {
        tokio::time::delay_for(Duration::from_secs(1)).await;
        let _ = bitcoin::mine_a_block(endpoint).await;
    }
}

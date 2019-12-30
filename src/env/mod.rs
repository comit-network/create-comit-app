use crate::{
    docker::{
        bitcoin::{self, BitcoindHttpEndpoint},
        delete_container, delete_network,
    },
    print_progress,
};
use futures::{
    future::{try_select, Either},
    pin_mut,
};
use std::time::Duration;
use tokio::time::delay_for;

mod start;

pub async fn start() {
    if crate::temp_fs::dir_exist().await {
        eprintln!("It seems that `create-comit-app start-env` is already running.\nIf it is not the case, run `create-comit-app force-clean-env` and try again.");
        ::std::process::exit(1);
    }

    let ctrl_c = tokio::signal::ctrl_c();
    let start_env = self::start::execute();

    pin_mut!(start_env);
    pin_mut!(ctrl_c);

    match try_select(start_env, ctrl_c).await {
        Ok(Either::Left((self::start::Environment { bitcoind, .. }, _))) => {
            tokio::spawn(new_miner(bitcoind.http_endpoint));
            println!("✓");
        }
        Ok(Either::Right((_signal, _))) => {
            print_progress!("🧹 Cleaning up");
            clean_up().await;
            println!("✓");
        }
        Err(_) => clean_up().await,
    }
}

async fn new_miner(endpoint: BitcoindHttpEndpoint) -> anyhow::Result<()> {
    loop {
        delay_for(Duration::from_secs(1)).await;
        bitcoin::mine_a_block(endpoint).await?;
    }
}

pub async fn clean_up() {
    let _ = delete_container("bitcoin").await;
    let _ = delete_container("ethereum").await;
    let _ = delete_container("cnd_0").await;
    let _ = delete_container("cnd_1").await;
    let _ = delete_network().await;

    if let Ok(path) = crate::temp_fs::dir_path() {
        let _ = tokio::fs::remove_dir_all(path).await;
    }
}

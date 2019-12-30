use crate::{
    docker::{delete_container, delete_network},
    print_progress,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::delay_for;

pub async fn handle_signal(terminate: Arc<AtomicBool>) {
    while !terminate.load(Ordering::Relaxed) {
        delay_for(Duration::from_millis(50)).await;
    }
    println!("Signal received, terminating...");
    print_progress!("🧹 Cleaning up");
    clean_up().await;
}

pub fn register_signals() -> anyhow::Result<Arc<AtomicBool>> {
    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&terminate))?;
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&terminate))?;

    #[cfg(not(windows))]
    {
        signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&terminate))?;
    }

    Ok(terminate)
}

pub async fn clean_up() {
    let _ = delete_container("bitcoin").await;
    let _ = delete_container("ethereum").await;
    let _ = delete_container("cnd_0").await;
    let _ = delete_container("cnd_1").await;
    let _ = delete_network().await;

    if let Ok(path) = crate::temp_fs::dir_path() {
        let _ = std::fs::remove_dir_all(path);
    }
}

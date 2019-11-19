use crate::docker::{delete_container, delete_network};
use crate::print_progress;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::prelude::stream;
use tokio::prelude::{Future, Stream};

pub fn handle_signal(terminate: Arc<AtomicBool>) -> impl Future<Item = (), Error = ()> {
    while !terminate.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50))
    }
    println!("Signal received, terminating...");
    print_progress!("🧹 Cleaning up");
    clean_up()
}

pub fn register_signals() -> anyhow::Result<Arc<AtomicBool>> {
    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&terminate))?;
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&terminate))?;
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&terminate))?;
    Ok(terminate)
}

pub fn clean_up() -> impl Future<Item = (), Error = ()> {
    delete_container("bitcoin")
        .then(|_| delete_container("ethereum"))
        .then(|_| {
            stream::iter_ok(vec![0, 1])
                .and_then(move |i| delete_container(format!("cnd_{}", i).as_str()))
                .collect()
        })
        .then(|_| delete_network())
        .then(|_| {
            let _ = crate::env::temp_fs::dir_path().map(std::fs::remove_dir_all);
            Ok(())
        })
        .map_err(|_: ()| ())
}

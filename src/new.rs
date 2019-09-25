use git2::Repository;
use std::fs::remove_dir_all;

const HELLO_SWAP_GIT: &str = "https://github.com/comit-network/hello-swap/";

pub fn new(name: String) {
    let _ = Repository::clone(HELLO_SWAP_GIT, name.clone())
        .map_err(|e| panic!("Failed to clone hello-swap: {:?}", e))
        .and_then(|_| remove_dir_all(format!("./{}/.git", name)))
        .map_err(|e| panic!("Failed to clean up hello-swap/.git folder: {:?}", e));
}

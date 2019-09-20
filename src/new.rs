use git2::Repository;

const HELLO_SWAP_GIT: &'static str = "https://github.com/comit-network/hello-swap/";

pub fn new() {
    let _ = Repository::clone(HELLO_SWAP_GIT, "./hello-swap")
        .map_err(|e| panic!("Failed to clone hello-swap: {:?}", e));
}

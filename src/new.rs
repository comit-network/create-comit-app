use git2::Repository;
use std::io::{Read, Write};

const HELLO_SWAP_ZIP: &str = "https://github.com/comit-network/hello-swap/archive/master.zip";

pub fn new(name: String) {
    let mut tempfile = tempfile::tempfile().expect("Could not create temp file");
    let mut buffer = Vec::new();
    ureq::get(HELLO_SWAP_ZIP)
        .call()
        .into_reader()
        .read_to_end(&mut buffer)
        .expect("Could not download hello swap zip file");
    tempfile
        .write_all(&buffer)
        .expect("Could not write hello swap zip file");
    unzip::Unzipper::new(tempfile, format!("./{}", name))
        .strip_components(1)
        .unzip()
        .expect("Could not unzip bundle");

    let _ = Repository::init(format!("./{}", name))
        .map_err(|e| panic!("Failed to create an empty Git repository: {:?}", e));
}

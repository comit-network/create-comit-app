use git2::{IndexAddOption, Repository, ResetType};
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
        .map_err(|e| panic!("Failed to create an empty Git repository: {:?}", e))
        .and_then(create_initial_commit)
        .map_err(|e| panic!("Failed to create initial commit: {:?}", e));
}

fn create_initial_commit(repo: Repository) -> Result<(), git2::Error> {
    let sig = repo.signature()?;

    let tree_id = {
        let mut index = repo.index()?;
        index.add_all(Vec::<String>::new(), IndexAddOption::DEFAULT, None)?;
        index.write_tree()?
    };

    let tree = repo.find_tree(tree_id)?;
    let commit = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

    // Otherwise the index is in a weird state after the commit.
    let obj = repo.find_object(commit, None)?;
    repo.reset(&obj, ResetType::Hard, None)?;

    Ok(())
}

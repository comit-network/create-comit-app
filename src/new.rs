use git2::IndexAddOption;
use git2::Repository;
use git2::ResetType;
use std::fs::remove_dir_all;

const HELLO_SWAP_GIT: &str = "https://github.com/comit-network/hello-swap/";

pub fn new(name: String) {
    let _ = Repository::clone(HELLO_SWAP_GIT, name.clone())
        .map_err(|e| panic!("Failed to clone hello-swap: {:?}", e))
        .and_then(|_| remove_dir_all(format!("./{}/.git", name)))
        .map_err(|e| panic!("Failed to clean up hello-swap/.git folder: {:?}", e))
        .and_then(|_| Repository::init(format!("./{}", name)))
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
    // TODO: Look into why this is the case
    let obj = repo.find_object(commit, None)?;
    repo.reset(&obj, ResetType::Hard, None)?;

    Ok(())
}

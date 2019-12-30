use std::{
    io,
    path::{Path, PathBuf},
};
use tar::Archive;
use tokio::{fs::File, prelude::*};

const NEW_PROJECT_ARCHIVE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/new_project.tar"));

pub async fn new(name: String) -> io::Result<()> {
    let mut archive = Archive::new(NEW_PROJECT_ARCHIVE);

    let path_to_write = PathBuf::from(&name);
    let package_json_path = path_to_write.join("package.json");

    tokio::task::spawn_blocking(move || archive.unpack(path_to_write)).await??;

    replace_project_name_in_file(package_json_path.as_path(), name.as_ref()).await?;

    println!("Your project `{}` has been created!\nNow, start your development environment by running `create-comit-app start-env`", name);

    Ok(())
}

async fn replace_project_name_in_file(path: &Path, name: &str) -> Result<(), io::Error> {
    let mut file = File::open(&path).await?;
    let mut data = String::new();
    file.read_to_string(&mut data).await?;
    drop(file);

    let to_replace = "new_project";
    let new_data = data.replace(to_replace, name);

    let mut destination = File::create(&path).await?;
    destination.write_all(new_data.as_bytes()).await?;

    Ok(())
}

use flate2::read::GzDecoder;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::Path;
use tar::Archive;

pub fn new(name: String, examples_archive: &[u8]) -> io::Result<()> {
    let tar = GzDecoder::new(examples_archive);
    let mut archive = Archive::new(tar);

    let path_to_write = Path::new(&name);
    archive.unpack(path_to_write)?;

    let package_json_path = path_to_write.join("package.json");
    replace_project_name_in_file(package_json_path.as_path(), name.as_ref())?;

    println!("Your project `{}` has been created!\nNow, start your development environment by running `create-comit-app start-env`", name);

    Ok(())
}

fn replace_project_name_in_file(path: &Path, name: &str) -> Result<(), io::Error> {
    let mut file = File::open(&path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    drop(file);

    let to_replace = "new_project";
    let new_data = data.replace(to_replace, name);

    let mut destination = File::create(&path)?;
    destination.write_all(new_data.as_bytes())?;

    Ok(())
}

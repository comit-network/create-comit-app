use crate::start_env::Error;
use std::path::PathBuf;

pub const DIR_NAME: &str = ".create-comit-app";
const ENV_FILE_NAME: &str = "env";

fn home() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot find the home directory, please ensure that $HOME is set on a unix system")
}

pub fn dir_path() -> PathBuf {
    home().join(DIR_NAME)
}

pub fn env_file_path() -> PathBuf {
    dir_path().join(ENV_FILE_NAME)
}

pub fn env_file_str() -> String {
    format!(
        "{}/{}/{}",
        home()
            .to_str()
            .expect("Could not get home directory as str"),
        DIR_NAME,
        ENV_FILE_NAME
    )
}

pub fn create_env_file() -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dir_path())?;
    std::fs::File::create(env_file_path())?;
    Ok(())
}

pub fn dir_exist() -> bool {
    std::fs::read_dir(dir_path()).is_ok()
}

pub fn temp_folder() -> Result<(PathBuf, String), Error> {
    let path = dir_path();

    std::fs::create_dir_all(&path).map_err(Error::CreateTmpFiles)?;
    let path = tempfile::tempdir_in(&path)
        .map_err(Error::CreateTmpFiles)?
        .into_path();
    let string = path.clone().to_str().ok_or(Error::PathToStr)?.to_string();
    Ok((path, string))
}

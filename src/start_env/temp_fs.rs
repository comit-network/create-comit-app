use crate::start_env::Error;
use std::path::PathBuf;

pub const DIR_NAME: &str = ".create-comit-app";
const ENV_FILE_NAME: &str = "env";

fn home() -> Result<PathBuf, Error> {
    dirs::home_dir().ok_or(Error::HomeDir)
}

pub fn dir_path() -> Result<PathBuf, Error> {
    Ok(home()?.join(DIR_NAME))
}

pub fn env_file_path() -> Result<PathBuf, Error> {
    Ok(dir_path()?.join(ENV_FILE_NAME))
}

pub fn env_file_str() -> Result<String, Error> {
    Ok(format!(
        "{}/{}/{}",
        home()?.to_str().ok_or(Error::PathToStr)?,
        DIR_NAME,
        ENV_FILE_NAME
    ))
}

pub fn create_env_file() -> Result<(), Error> {
    std::fs::create_dir_all(dir_path()?).map_err(Error::CreateTmpFiles)?;
    std::fs::File::create(env_file_path()?).map_err(Error::CreateTmpFiles)?;
    Ok(())
}

pub fn dir_exist() -> bool {
    if let Ok(dir_path) = dir_path() {
        std::fs::read_dir(dir_path).is_ok()
    } else {
        false
    }
}

pub fn temp_folder() -> Result<(PathBuf, String), Error> {
    let path = dir_path()?;

    std::fs::create_dir_all(&path).map_err(Error::CreateTmpFiles)?;
    let path = tempfile::tempdir_in(&path)
        .map_err(Error::CreateTmpFiles)?
        .into_path();
    let string = path.clone().to_str().ok_or(Error::PathToStr)?.to_string();
    Ok((path, string))
}

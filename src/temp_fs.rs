use anyhow::Context;
use std::path::PathBuf;

pub const DIR_NAME: &str = ".create-comit-app";
const ENV_FILE_NAME: &str = "env";

fn home() -> anyhow::Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow::anyhow!("unable to determine home directory"))
}

pub fn dir_path() -> anyhow::Result<PathBuf> {
    Ok(home()?.join(DIR_NAME))
}

pub fn env_file_path() -> anyhow::Result<PathBuf> {
    Ok(dir_path()?.join(ENV_FILE_NAME))
}

pub fn create_env_file() -> anyhow::Result<String> {
    let _ = ensure_cca_directory()?;

    let env_file_path = env_file_path()?;
    std::fs::File::create(&env_file_path)
        .with_context(|| format!("failed to create file {}", env_file_path.display()))?;
    Ok(format!(
        "{}/{}/{}",
        home()?.display(),
        DIR_NAME,
        ENV_FILE_NAME
    ))
}

pub fn dir_exist() -> bool {
    if let Ok(dir_path) = dir_path() {
        std::fs::read_dir(dir_path).is_ok()
    } else {
        false
    }
}

pub fn temp_folder() -> anyhow::Result<PathBuf> {
    let path = ensure_cca_directory()?;

    let path = tempfile::tempdir_in(&path)
        .with_context(|| format!("failed to create temporary directory in {}", path.display()))?
        .into_path();

    Ok(path)
}

fn ensure_cca_directory() -> anyhow::Result<PathBuf> {
    let cca_path = dir_path()?;
    std::fs::create_dir_all(&cca_path)
        .with_context(|| format!("failed to create directory: {}", cca_path.display()))?;

    Ok(cca_path)
}

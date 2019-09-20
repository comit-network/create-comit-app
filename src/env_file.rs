use envfile::EnvFile;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

const ENV_FILE: &str = ".env";
const LOCK_FILE_SUFFIX: &str = ".lock";
const LOCK_FILE_CONTENT: &[u8] =
    b"# This lock file is to ensure that no two create-comit-app updates the .env file.";

pub enum Error {
    IsLocked,
    CannotWriteLock(io::Error),
    CannotRemoveLock(io::Error),
    CannotUpdateEnvFile(io::Error),
}

pub trait LockUpdateWrite {
    fn lock_update_write(&mut self, key: &str, value: &str) -> Result<(), Error>;
}

fn lock_path(envfile: &EnvFile) -> PathBuf {
    // TODO: Remove unwrap
    let mut lock_path = envfile.path.as_os_str().to_str().unwrap().to_string();
    lock_path.push_str(LOCK_FILE_SUFFIX);
    Path::new(&lock_path).to_path_buf()
}

fn lock(envfile: &EnvFile) -> Result<(), Error> {
    let lock_path = lock_path(envfile);

    if lock_path.exists() {
        Err(Error::IsLocked)
    } else {
        let mut file = File::create(lock_path)?;
        file.write_all(LOCK_FILE_CONTENT)?;
        Ok(())
    }
}

fn unlock(envfile: &EnvFile) -> Result<(), Error> {
    let lock_path = lock_path(envfile);
    std::fs::remove_file(lock_path).map_err(|e| Error::CannotRemoveLock(e))
}

impl LockUpdateWrite for EnvFile {
    fn lock_update_write(&mut self, key: &str, value: &str) -> Result<(), Error> {
        lock(self)
            .and(
                self.update(key, value)
                    .write()
                    .map_err(Error::CannotUpdateEnvFile),
            )
            .and(unlock(self))
    }
}

impl From<io::Error> for Error {
    fn from(io_error: io::Error) -> Self {
        Error::CannotWriteLock(io_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_lock_and_unlock_twice() {
        let temp_file = tempfile::Builder::new().suffix(".env").tempfile().unwrap();
        let mut env_file = EnvFile::new(temp_file.path()).unwrap();

        let res = env_file.lock_update_write("key", "value");
        assert!(res.is_ok());

        let res = env_file.lock_update_write("key", "value");
        assert!(res.is_ok());
    }

    #[test]
    fn cannot_lock() {
        let temp_file = tempfile::Builder::new().suffix(".env").tempfile().unwrap();
        let mut env_file = EnvFile::new(temp_file.path()).unwrap();

        let mut lock_file = File::create(lock_path(&env_file)).unwrap();
        lock_file.write_all(b"").unwrap();

        let res = env_file.lock_update_write("key", "value");
        assert!(res.is_err());
    }
}

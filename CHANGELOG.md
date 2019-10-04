# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 - 2019-10-04

### Added
- Ensure that all temporary folders are cleaned up when shutting down start-env.
- Display clear message if start-env is started twice on the same machine.

### Changed
- Move start-env env file to `~/.create-comit-app/env` so that the user does not have to start it from inside the project folder.
- If a signal (e.g. CTRL-C) is sent while `start-env` is booting, it waits for the action in progress (e.g., starting docker container) and then stops and properly cleans up the environment. 
- Improve code quality to avoid artifacts remaining after a failure happens while booting start-env.
- Migrate to comit-rs 0.3.0. Only the `cnd` docker is now needed!
- Move the env file from `./.env` to `~/.create-comit-app/env` to improve usability.
- Only pull docker images if they are not present locally, this allows offline usage of the `start-env` command.

## 0.1.0 - 2019-09-26

First Release 🎉

[Unreleased]: https://github.com/comit-network/create-comit-app/compare/0.2.0...HEAD
[0.2.0]: https://github.com/comit-network/create-comit-app/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/comit-network/create-comit-app/releases/tag/0.1.0

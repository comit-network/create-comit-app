RUSTUP = rustup

TOOLCHAIN = $(shell cat rust-toolchain)
CARGO = $(RUSTUP) run --install $(TOOLCHAIN) cargo --color always

NIGHTLY_TOOLCHAIN = "nightly-2019-07-31"
CARGO_NIGHTLY = $(RUSTUP) run --install $(NIGHTLY_TOOLCHAIN) cargo --color always

# cannot use the unix-socket to talk to the docker daemon on windows
ifeq ($(OS),Windows_NT)
    BUILD_ARGS = --no-default-features --features windows
    TEST_ARGS = --no-default-features --features windows
    INSTALL_ARGS = --no-default-features --features windows
endif

.PHONY: install_rust install_rust_nightly install_clippy install_rustfmt install_tomlfmt install clean all format build build_debug release clippy test doc check_format e2e_scripts e2e

default: build

install_rust:
	$(RUSTUP) install $(TOOLCHAIN)

install_rust_nightly:
	$(RUSTUP) install $(NIGHTLY_TOOLCHAIN)

## Dev environment

install_clippy: install_rust
	$(RUSTUP) component list --installed --toolchain $(TOOLCHAIN) | grep -q clippy || $(RUSTUP) component add clippy --toolchain $(TOOLCHAIN)

# need nightly toolchain to get access to `merge_imports`
install_rustfmt: install_rust_nightly
	$(RUSTUP) component list --installed --toolchain $(NIGHTLY_TOOLCHAIN) | grep -q rustfmt || $(RUSTUP) component add rustfmt --toolchain $(NIGHTLY_TOOLCHAIN)

install_tomlfmt: install_rust
	$(CARGO) --list | grep -q tomlfmt || $(CARGO) install cargo-tomlfmt

## User install

install:
	$(CARGO) install --force --path create-comit-app $(INSTALL_ARGS)
	$(CARGO) install --force --path comit-scripts $(INSTALL_ARGS)

clean:
	$(CARGO) clean

## Development tasks

all: format build_debug clippy test doc e2e_scripts

format: install_rustfmt install_tomlfmt
	$(CARGO_NIGHTLY) fmt
	$(CARGO) tomlfmt -p Cargo.toml
	$(CARGO) tomlfmt -p create/Cargo.toml
	$(CARGO) tomlfmt -p scripts/Cargo.toml

build: build_debug

build_debug:
	cd ./create; $(CARGO) build --all-targets $(BUILD_ARGS)
	cd ./scripts; $(CARGO) build --all-targets $(BUILD_ARGS)

release: release_create release_scripts

release_create:
	cd ./create; $(CARGO) build --all-targets --release $(BUILD_ARGS)

release_scripts:
	cd ./scripts; $(CARGO) build --all-targets --release $(BUILD_ARGS)

clippy: install_clippy
	$(CARGO) clippy --all-targets -- -D warnings

test:
	cd ./create; $(CARGO) test $(TEST_ARGS)
	cd ./scripts; $(CARGO) test $(TEST_ARGS)

doc:
	$(CARGO) doc

check_format: install_rustfmt install_tomlfmt
	$(CARGO_NIGHTLY) fmt -- --check
	$(CARGO) tomlfmt -d -p Cargo.toml
	$(CARGO) tomlfmt -d -p create/Cargo.toml
	$(CARGO) tomlfmt -d -p scripts/Cargo.toml

yarn_install_all:
	cd ./scripts/npm; yarn install
	cd ./create/npm; yarn install
	cd ./create/new_project/examples/btc_eth; yarn install
	cd ./create/new_project/examples/erc20_btc; yarn install
	cd ./create/new_project/examples/separate_apps; yarn install

yarn_upgrade_all:
	cd ./scripts/npm; yarn upgrade
	cd ./create/npm; yarn upgrade
	cd ./create/new_project/examples/btc_eth; yarn upgrade
	cd ./create/new_project/examples/erc20_btc; yarn upgrade
	cd ./create/new_project/examples/separate_apps; yarn upgrade

yarn_fix_all:
	cd ./scripts/npm; yarn run fix
	cd ./create/npm; yarn run fix
	cd ./create/new_project/examples/btc_eth; yarn run fix
	cd ./create/new_project/examples/erc20_btc; yarn run fix
	cd ./create/new_project/examples/separate_apps; yarn run fix

e2e_scripts:
	./tests/new.sh
	./tests/start_env.sh
	./tests/force_clean_env.sh
	./tests/btc_eth.sh

e2e: build_debug e2e_scripts

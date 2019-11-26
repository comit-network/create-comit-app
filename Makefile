RUSTUP = rustup

TOOLCHAIN = $(shell cat rust-toolchain)
CARGO = $(RUSTUP) run --install $(TOOLCHAIN) cargo --color always

NIGHTLY_TOOLCHAIN = "nightly-2019-07-31"
CARGO_NIGHTLY = $(RUSTUP) run --install $(NIGHTLY_TOOLCHAIN) cargo --color always

ifneq ($(OS),Windows_NT)
    BUILD_ARGS := --features shiplift/unix-socket
endif

build: build_debug

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
	$(CARGO) install --force --path .

clean:
	$(CARGO) clean

## Development tasks

all: format build_debug clippy test doc e2e_scripts

format: install_rustfmt install_tomlfmt
	$(CARGO_NIGHTLY) fmt
	$(CARGO) tomlfmt -p Cargo.toml

build: build_debug

build_debug:
	$(CARGO) build --all --all-targets $(BUILD_ARGS)

clippy: install_clippy
	$(CARGO) clippy --all-targets -- -D warnings

test:
	$(CARGO) test --all

doc:
	$(CARGO) doc

check_format: install_rustfmt install_tomlfmt
	$(CARGO_NIGHTLY) fmt -- --check
	$(CARGO) tomlfmt -d -p Cargo.toml

e2e_scripts:
	./tests/new.sh
	./tests/start_env.sh
	./tests/force_clean_env.sh
	./tests/btc_eth.sh

e2e: build_debug e2e_scripts

RUSTUP = rustup
TOOLCHAIN = $(shell cat rust-toolchain)
CARGO = $(RUSTUP) run --install $(TOOLCHAIN) cargo --color always

build: build_debug

## Dev environment

install_clippy:
	$(RUSTUP) component list --installed --toolchain $(TOOLCHAIN) | grep -q clippy || $(RUSTUP) component add clippy --toolchain $(TOOLCHAIN)

install_rustfmt:
	$(RUSTUP) component list --installed --toolchain $(TOOLCHAIN) | grep -q rustfmt || $(RUSTUP) component add rustfmt --toolchain $(TOOLCHAIN)

install_tomlfmt:
	$(CARGO) --list | grep -q tomlfmt || $(CARGO) install cargo-tomlfmt

## User install

install:
	$(CARGO) install --force --path .

clean:
	$(CARGO) clean

## Development tasks

all: format build_debug clippy test doc

format: install_rustfmt install_tomlfmt
	$(CARGO) fmt
	$(CARGO) tomlfmt -p Cargo.toml

build: build_debug

build_debug:
	$(CARGO) build --all --all-targets

clippy: install_clippy
	$(CARGO) clippy --all-targets -- -D warnings

test:
	$(CARGO) test --all

doc:
	$(CARGO) doc

check_format: install_rustfmt install_tomlfmt
	$(CARGO) fmt -- --check
	$(CARGO) tomlfmt -d -p Cargo.toml
